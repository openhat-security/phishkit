---
layout: home
title: phishkit
titleTemplate: false

hero:
  name: phishkit
  text: Authorized phishing assessments, end to end
  tagline: evilginx AiTM plus a native email campaign engine in one local desktop app — deep enough for operators, guided enough for a business user.
  actions:
    - theme: brand
      text: Quick start
      link: /guide/quick-start
    - theme: alt
      text: Authorized use
      link: /guide/authorized-use
    - theme: alt
      text: View on GitHub
      link: https://github.com/openhat-security/phishkit

features:
  - title: One end-to-end workflow
    details: Assessment → Target → Phishlet/Proxy → Lure → Template → Recipients → Campaign → Results → Session, all in the desktop app.
  - title: Native email campaign engine
    details: Draft/review/test/launch campaigns with your SMTP or ESP, scheduling and send windows, delivered/opened/clicked/bounced tracking, and reporting.
  - title: Real session capture
    details: evilginx captures what an attacker would actually get, with deterministic attribution back to the campaign attempt and a focused Sessions view.
  - title: Dual audience
    details: A guided wizard and curated presets with safe defaults for business users, layered over full Advanced controls for expert operators.
---

## For authorized assessments only

phishkit drives an adversary-in-the-middle proxy, sends email, and handles
captured credentials and live session tokens. Use it only with **explicit
written authorization** from the owner of the targeted systems and people.

Read [authorized use](/guide/authorized-use) and the
[threat model](/reference/threat-model) before you run anything. The supported
product is the desktop app under `apps/desktop/` (plus the headless CLI).

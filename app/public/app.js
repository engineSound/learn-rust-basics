// Front end shared by learn-nodejs and learn-rust (byte-for-byte identical in both repos).
// API (docs/SHARED-FORMAT.md, section 2):
//   GET  /api/info                      -> { track, title, accent, tutor: { enabled, reason } }
//   GET  /api/lessons                   -> [{ slug, id, title, minutes, concepts, kind, for, topic, stats }]
//   GET  /api/lessons/:slug             -> { slug, meta, body, file, code, saved, stats }
//   PUT  /api/lessons/:slug/code        (text body) -> { saved: true }
//   DELETE /api/lessons/:slug/code      -> { code, stats, next }  (back to the starter)
//   POST /api/lessons/:slug/run         (text body) -> run result + { stats, next }
//   POST /api/lessons/:slug/feedback    (easy|right|hard) -> { stats, next }
//   POST /api/lessons/:slug/log         ({ kind: "hint"|"predict", payload }) -> { ok }
//   POST /api/lessons/:slug/hint        ({ code, question }) -> { hint }       (Claude tutor)
//   POST /api/lessons/:slug/visualize   ({ code }) -> { visual, verified, attempts } (Claude tutor)
//   POST /api/lessons/:slug/drill       -> { slug, title, targets, stats, next } (Claude tutor)
//   POST /api/snippet                   ({ code }) -> { output, ok, errors, timedOut }
//   GET  /api/profile                   -> { profile, markdown, next }
//   GET  /api/history                   -> [event] newest first, without code
(function () {
  'use strict';
  var $ = function (id) { return document.getElementById(id); };
  var state = { info: null, lessons: [], profile: null, next: null, current: null, history: [], tutor: { enabled: false } };
  var code = $('code'), gutter = $('gutter'), saveTimer = null;

  // ---------- helpers ----------
  function api(method, path, body, json) {
    var headers = body == null ? {} : { 'Content-Type': json ? 'application/json' : 'text/plain; charset=utf-8' };
    return fetch(path, { method: method, body: body == null ? undefined : (json ? JSON.stringify(body) : body), headers: headers })
      .then(function (r) {
        return r.json().then(function (data) {
          if (!r.ok) throw new Error(data.error || ('HTTP ' + r.status));
          return data;
        });
      });
  }
  function esc(s) {
    return String(s == null ? '' : s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }
  function md(text) { return window.renderMarkdown(text || ''); }
  function store(key, value) { try { localStorage.setItem('lab.' + key, JSON.stringify(value)); } catch (e) { /* private mode */ } }
  function load(key, fallback) { try { var v = localStorage.getItem('lab.' + key); return v == null ? fallback : JSON.parse(v); } catch (e) { return fallback; } }
  function plural(n, one, many) { return n + ' ' + (n === 1 ? one : (many || one + 's')); }
  function dayKey(ts) { var d = new Date(ts); return d.getFullYear() + '-' + (d.getMonth() + 1) + '-' + d.getDate(); }
  function lessonBySlug(slug) { return state.lessons.filter(function (l) { return l.slug === slug; })[0]; }
  function lessonById(id) { return state.lessons.filter(function (l) { return l.id === id; })[0]; }
  function cores() { return state.lessons.filter(function (l) { return l.kind === 'core'; }); }
  function solved(l) { return !!(l && l.stats && l.stats.first_pass_run); }
  var KIND = { review: 'Review', drill: 'Practice', stretch: 'Challenge', lesson: 'Lesson', core: 'Lesson' };

  // ---------- routing: #today, #learn/<slug>, #history, #profile ----------
  function route() {
    var hash = location.hash.replace(/^#/, '') || 'today';
    var parts = hash.split('/');
    var view = ['today', 'learn', 'history', 'profile'].indexOf(parts[0]) >= 0 ? parts[0] : 'today';
    ['today', 'learn', 'history', 'profile'].forEach(function (v) { $('view-' + v).hidden = v !== view; });
    Array.prototype.forEach.call(document.querySelectorAll('.tabs a'), function (a) {
      a.classList.toggle('active', a.getAttribute('data-view') === view);
    });
    if (view === 'today') renderToday();
    if (view === 'history') renderHistory();
    if (view === 'profile') renderProfile();
    if (view === 'learn') {
      var slug = parts[1] || (state.current && state.current.slug) || (state.next && state.next.slug) || load('lesson', null) || (cores()[0] || {}).slug;
      if (slug && (!state.current || state.current.slug !== slug)) openLesson(slug);
      else renderOutline();
    }
    window.scrollTo(0, 0);
  }
  function go(hash) { if (location.hash === '#' + hash) route(); else location.hash = hash; }
  window.addEventListener('hashchange', route);

  function refresh() {
    return Promise.all([api('GET', '/api/lessons'), api('GET', '/api/profile'), api('GET', '/api/history')]).then(function (r) {
      state.lessons = r[0]; state.profile = r[1].profile; state.next = r[1].next; state.history = r[2];
      renderTop();
    });
  }

  function renderTop() {
    var c = cores(), done = c.filter(solved).length;
    var streak = state.profile ? state.profile.sessions.streak_days : 0;
    $('top-streak').textContent = streak ? plural(streak, 'day') + ' streak' : '';
    $('top-progress').textContent = done + ' / ' + c.length + ' lessons';
  }

  // ---------- Today ----------
  function todayPlan() {
    var plan = [], seen = {};
    function add(item) { if (item && item.slug && !seen[item.slug]) { seen[item.slug] = 1; plan.push(item); } }
    var n = state.next;
    if (n && n.slug && n.kind !== 'lesson') add({ slug: n.slug, kind: n.kind, title: n.title, reason: n.reason });
    state.lessons.filter(function (l) { return l.kind === 'core' && l.stats && l.stats.due_now; }).forEach(function (l) {
      add({ slug: l.slug, kind: 'review', title: l.title, reason: 'due for review: start over and solve it from memory' });
    });
    var current = cores().filter(function (l) { return !solved(l); })[0];
    if (current) add({ slug: current.slug, kind: 'lesson', title: current.title, reason: current.stats && current.stats.runs ? 'continue where you left off' : 'your next new lesson' });
    return plan.slice(0, 3).map(function (p) { var l = lessonBySlug(p.slug); p.minutes = (l && l.minutes) || 10; p.topic = l && l.topic; return p; });
  }

  function renderToday() {
    var now = new Date(), h = now.getHours();
    $('today-date').textContent = now.toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric' });
    $('today-greeting').textContent = h < 12 ? 'Good morning.' : h < 18 ? 'Good afternoon.' : 'Good evening.';
    var plan = todayPlan(), minutes = plan.reduce(function (s, p) { return s + p.minutes; }, 0);
    if (!plan.length) {
      $('today-lead').textContent = state.next && state.next.reason ? 'You’re all caught up: ' + state.next.reason + '.' : 'You’re all caught up.';
      $('plan').innerHTML = '';
    } else {
      $('today-lead').textContent = 'Here’s your plan for today: about ' + minutes + ' minutes.';
      $('plan').innerHTML = plan.map(function (p, i) {
        return '<li class="plan-item' + (i === 0 ? ' first' : '') + '">' +
          '<span class="plan-kind kind-' + esc(p.kind) + '">' + esc(KIND[p.kind] || 'Lesson') + '</span>' +
          '<div class="plan-text"><strong>' + esc(p.title) + '</strong><span>' + esc(p.reason.charAt(0).toUpperCase() + p.reason.slice(1)) + (p.topic ? ' · ' + esc(p.topic) : '') + '</span></div>' +
          '<span class="plan-min">' + p.minutes + ' min</span>' +
          '<a class="' + (i === 0 ? 'btn primary' : 'btn') + '" href="#learn/' + esc(p.slug) + '">' + (i === 0 ? 'Start' : 'Open') + '</a></li>';
      }).join('');
    }
    $('today-note').textContent = 'The plan adapts to you: reviews return just before you’d forget, and how each exercise felt decides what comes next.';

    var c = cores(), done = c.filter(solved).length;
    $('ring-done').textContent = done;
    $('ring-total').textContent = 'of ' + plural(c.length, 'lesson');
    var circ = 2 * Math.PI * 52;
    $('ring-fg').style.strokeDasharray = circ;
    $('ring-fg').style.strokeDashoffset = circ * (1 - (c.length ? done / c.length : 0));
    var s = state.profile ? state.profile.sessions : { streak_days: 0, minutes_total: 0 };
    var days = {};
    state.history.forEach(function (e) { days[dayKey(e.ts)] = 1; });
    $('stat-streak').textContent = plural(s.streak_days, 'day');
    $('stat-days').textContent = Object.keys(days).length;
    $('stat-time').textContent = s.minutes_total + ' min';
    $('today-concepts').innerHTML = conceptBars(5, 'What you know best and least');
    var firstOpen = c.filter(function (l) { return !solved(l); })[0];
    $('path').innerHTML = c.map(function (l, i) {
      var cls = solved(l) ? (l.stats.due_now ? 'due' : 'done') : l === firstOpen ? 'now' : 'todo';
      return '<li class="' + cls + '"><a href="#learn/' + esc(l.slug) + '"><span class="dot">' + (cls === 'done' ? '✓' : cls === 'due' ? '↻' : i + 1) + '</span>' +
        '<span class="path-text"><b>' + esc(l.title) + '</b><small>' + esc(l.topic || '') + '</small></span></a></li>';
    }).join('');
  }

  function conceptBars(limit, title) {
    var cs = (state.profile ? state.profile.concepts : []).filter(function (c) { return c.mastery !== null; });
    if (!cs.length) return '<h3>' + esc(title) + '</h3><p class="muted small">Solve your first exercise to see how well you know each idea.</p>';
    cs = cs.slice().sort(function (a, b) { return b.mastery - a.mastery; });
    if (limit && cs.length > limit) cs = cs.slice(0, Math.ceil(limit / 2)).concat(cs.slice(-Math.floor(limit / 2)));
    return '<h3>' + esc(title) + '</h3><ul class="bars">' + cs.map(function (c) {
      var pct = Math.round(c.mastery * 100), level = c.mastery >= 0.7 ? 'good' : c.mastery >= 0.45 ? 'mid' : 'low';
      return '<li><span>' + esc(c.name.replace(/-/g, ' ')) + '</span><span class="bar"><span class="' + level + '" style="width:' + pct + '%"></span></span><b>' + pct + '%</b></li>';
    }).join('') + '</ul>';
  }

  // ---------- Learn: outline ----------
  function statusIcon(l) {
    var st = l.stats || {};
    if (st.due_now) return '<span class="st due" title="Due for review">↻</span>';
    if (st.first_pass_run) return '<span class="st done" title="Solved">✓</span>';
    if (st.runs) return '<span class="st started" title="Started">•</span>';
    return '<span class="st new"></span>';
  }
  $('rail-toggle').addEventListener('click', function () {
    var rail = document.querySelector('.rail'), open = !rail.classList.contains('open');
    rail.classList.toggle('open', open);
    $('rail-toggle').setAttribute('aria-expanded', String(open));
  });
  // Wide screens: the lesson list can be folded away (remembered); narrow screens use the toggle above.
  function setRailHidden(hidden) {
    $('view-learn').classList.toggle('rail-hidden', hidden);
    var b = $('rail-collapse');
    b.textContent = hidden ? '»' : '«';
    b.title = hidden ? 'Show the lesson list' : 'Hide the lesson list';
    b.setAttribute('aria-expanded', String(!hidden));
    store('railHidden', hidden);
  }
  $('rail-collapse').addEventListener('click', function () {
    setRailHidden(!$('view-learn').classList.contains('rail-hidden'));
  });
  setRailHidden(load('railHidden', true));
  function renderOutline() {
    var c = cores();
    document.querySelector('.rail').classList.remove('open');
    $('rail-toggle').setAttribute('aria-expanded', 'false');
    $('rail-name').textContent = (state.info ? state.info.trackName : '') + ' path';
    $('rail-count').textContent = c.filter(solved).length + ' / ' + c.length;
    $('outline').innerHTML = c.map(function (l, i) {
      var kids = state.lessons.filter(function (k) { return k.for === l.id; });
      var active = state.current && state.current.slug === l.slug ? ' active' : '';
      return '<li><a class="ol-item' + active + '" href="#learn/' + esc(l.slug) + '">' +
        '<span class="ol-num">' + String(i + 1).padStart(2, '0') + '</span>' +
        '<span class="ol-text"><span class="ol-title">' + esc(l.title) + '</span><span class="ol-sub">' + esc(l.topic || (l.concepts || []).slice(0, 2).join(', ')) + ' · ' + (l.minutes || '?') + ' min</span></span>' +
        statusIcon(l) + '</a>' +
        (kids.length ? '<ul class="ol-kids">' + kids.map(function (k) {
          var a = state.current && state.current.slug === k.slug ? ' active' : '';
          return '<li><a class="ol-kid' + a + '" href="#learn/' + esc(k.slug) + '"><span>' + (k.kind === 'stretch' ? 'Challenge' : 'Practice') + ': ' + esc(k.title) + '</span>' + statusIcon(k) + '</a></li>';
        }).join('') + '</ul>' : '') + '</li>';
    }).join('');
  }

  // ---------- Learn: one lesson ----------
  function sections(body) {
    var out = { _order: [] }, cur = '_intro', buf = [];
    body.split('\n').forEach(function (line) {
      var m = /^## (.+)$/.exec(line);
      if (m) { out[cur] = buf.join('\n'); out._order.push(cur); cur = m[1].trim().toLowerCase(); buf = []; }
      else buf.push(line);
    });
    out[cur] = buf.join('\n'); out._order.push(cur);
    out._intro = (out._intro || '').replace(/^# .*$/m, '').trim();
    return out;
  }
  var KNOWN = ['_intro', 'learn the idea', 'the idea', 'recall', 'quick check', 'your turn', 'exercise', 'example', 'hints', 'review'];

  function openLesson(slug) {
    store('lesson', slug);
    return api('GET', '/api/lessons/' + encodeURIComponent(slug)).then(function (l) {
      state.current = l;
      var meta = l.meta, s = sections(l.body), list = lessonBySlug(slug) || {};
      var c = cores(), idx = c.indexOf(lessonById(meta.kind && meta.kind !== 'core' ? meta['for'] : meta.id));
      var where = meta.kind === 'drill' ? 'Practice' : meta.kind === 'stretch' ? 'Challenge' : 'Lesson ' + (idx + 1) + ' of ' + c.length;
      $('ls-eyebrow').textContent = where + (meta.topic ? ' · ' + meta.topic : '') + ' · ' + (meta.minutes || '?') + ' min';
      $('ls-title').textContent = meta.title;
      document.title = meta.title + ' · ' + state.info.title;

      // Step 1: the idea (plus any extra sections the lesson has).
      var extra = s._order.filter(function (k) { return KNOWN.indexOf(k) < 0; })
        .map(function (k) { return '### ' + k.charAt(0).toUpperCase() + k.slice(1) + '\n\n' + s[k]; }).join('\n\n');
      var ideaKey = s['learn the idea'] != null ? 'learn the idea' : s['the idea'] != null ? 'the idea' : s.recall != null ? 'recall' : null;
      $('idea-heading').textContent = ideaKey === 'recall' ? 'Recall' : 'The idea';
      $('ls-idea').innerHTML = md([s._intro, ideaKey ? s[ideaKey] : '', extra].filter(Boolean).join('\n\n'));
      var hasCheck = !!(s['quick check'] && s['quick check'].trim());
      $('step-check').hidden = !hasCheck;
      $('ls-check').innerHTML = hasCheck ? md(s['quick check']) : '';
      $('turn-num').textContent = hasCheck ? '3' : '2';

      // Step 3: your turn.
      $('ls-turn').innerHTML = md(s['your turn'] || s.exercise || 'Make the tests pass.');
      $('ls-example-box').hidden = !s.example;
      $('ls-example').innerHTML = s.example ? md(s.example) : '';
      $('file-name').textContent = l.file;
      code.value = l.code; updateGutter(); code.scrollTop = 0;
      $('save-state').textContent = l.saved ? 'Your saved work' : 'Starter code';

      state.hints = ((s.hints || '').match(/^\s*\d+\.\s+.+$/gm) || []).map(function (h) { return h.replace(/^\s*\d+\.\s+/, ''); });
      state.review = (s.review || '').trim();
      hydrate($('view-learn'));
      renderStatus(l.stats);
      $('results').innerHTML = '';
      $('after').innerHTML = '';
      renderHelp(false);
      if (l.stats && l.stats.due_now) {
        $('results').innerHTML = '<div class="note">↻ <b>Time to review.</b> Press <b>Start over</b>, then solve it from memory. It’s fine to peek at the idea first.</div>';
      }
      renderOutline();
    }).catch(function (e) { $('ls-title').textContent = 'Couldn’t open this lesson'; $('ls-idea').innerHTML = '<p>' + esc(e.message) + '</p>'; });
  }

  function renderStatus(st) {
    var text = !st || !st.runs ? '' : st.due_now ? '↻ Review due' : st.first_pass_run ? '✓ Solved on run ' + st.first_pass_run : plural(st.runs, 'run') + ' so far';
    $('ls-status').innerHTML = text ? '<span class="chip' + (st.first_pass_run && !st.due_now ? ' ok' : '') + '">' + esc(text) + '</span>' : '';
    if (state.current) {
      var l = lessonBySlug(state.current.slug);
      if (l) l.stats = st;
      renderOutline(); renderTop();
    }
  }

  // ---------- widgets inside lesson text ----------
  function hydrate(root) {
    if (window.hydrateVisuals) window.hydrateVisuals(root);
    Array.prototype.forEach.call(root.querySelectorAll('.try'), function (el) {
      var src = decodeURIComponent(el.getAttribute('data-code'));
      el.innerHTML = '<div class="try-bar"><span>Try it: change the code and run it</span><button type="button" class="btn small">▶ Run</button></div>' +
        '<textarea class="try-code" spellcheck="false" rows="' + Math.min(14, src.split('\n').length + 1) + '"></textarea><pre class="try-out" hidden></pre>';
      var ta = el.querySelector('textarea'), out = el.querySelector('.try-out'), btn = el.querySelector('button');
      ta.value = src;
      function run() {
        btn.disabled = true; out.hidden = false; out.className = 'try-out'; out.textContent = 'Running…';
        runSnippet(ta.value).then(function (r) { out.className = 'try-out' + (r.ok ? '' : ' bad'); out.textContent = r.text; })
          .then(function () { btn.disabled = false; });
      }
      btn.addEventListener('click', run);
      ta.addEventListener('keydown', function (e) {
        if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') { e.preventDefault(); e.stopPropagation(); run(); }
        if (e.key === 'Tab' && !e.shiftKey) { e.preventDefault(); insertIndent(ta); }
      });
    });
    Array.prototype.forEach.call(root.querySelectorAll('.predict'), function (el, qi) {
      var src = decodeURIComponent(el.getAttribute('data-code'));
      var options = decodeURIComponent(el.getAttribute('data-options')).split('|').map(function (o) { return o.trim(); }).filter(Boolean);
      var answer = decodeURIComponent(el.getAttribute('data-answer')).trim(), why = decodeURIComponent(el.getAttribute('data-why'));
      el.innerHTML = '<pre><code>' + esc(src) + '</code></pre><p class="q">What will this print? Pick one before you run it.</p>' +
        '<div class="options">' + (options.length ? options : ['(type your answer)']).map(function (o) {
          return options.length ? '<button type="button" class="opt" data-v="' + esc(o) + '">' + esc(o) + '</button>' : '<input type="text" class="opt-input" placeholder="Your prediction"><button type="button" class="btn small opt-check">Check</button>';
        }).join('') + '</div><div class="verdict" hidden></div>';
      function decide(guess) {
        if (el.classList.contains('answered')) return;
        el.classList.add('answered');
        var ok = guess.trim().toLowerCase() === answer.toLowerCase();
        Array.prototype.forEach.call(el.querySelectorAll('.opt'), function (b) {
          if (b.getAttribute('data-v').toLowerCase() === answer.toLowerCase()) b.classList.add('right');
          else if (b.getAttribute('data-v') === guess) b.classList.add('wrong');
          b.disabled = true;
        });
        var v = el.querySelector('.verdict');
        v.hidden = false;
        v.innerHTML = '<p><b>' + (ok ? '✓ Right.' : '✗ Not quite. It prints ' + esc(answer) + '.') + '</b> ' + md(why).replace(/^<p>|<\/p>$/g, '') + '</p>' +
          '<button type="button" class="btn small">▶ Run it to see</button><pre class="try-out" hidden></pre>';
        var out = v.querySelector('.try-out');
        v.querySelector('button').addEventListener('click', function () {
          out.hidden = false; out.textContent = 'Running…';
          runSnippet(src).then(function (r) { out.className = 'try-out' + (r.ok ? '' : ' bad'); out.textContent = r.text; });
        });
        if (state.current) api('POST', '/api/lessons/' + state.current.slug + '/log', { kind: 'predict', payload: { question: qi + 1, correct: ok } }, true).catch(function () {});
      }
      Array.prototype.forEach.call(el.querySelectorAll('.opt'), function (b) { b.addEventListener('click', function () { decide(b.getAttribute('data-v')); }); });
      var check = el.querySelector('.opt-check');
      if (check) check.addEventListener('click', function () { decide(el.querySelector('.opt-input').value); });
    });
  }
  function runSnippet(src) {
    return api('POST', '/api/snippet', { code: src }, true).then(function (r) {
      if (r.timedOut) return { ok: false, text: 'Stopped: it ran for too long (an endless loop?).' };
      if (r.errors && r.errors.length) return { ok: false, text: r.errors.map(function (e) { return (e.code ? e.code + ': ' : '') + e.title + '\n' + e.message; }).join('\n\n') };
      return { ok: r.ok, text: r.output.trim() || '(no output)' };
    }).catch(function (e) { return { ok: false, text: e.message }; });
  }

  // ---------- editor ----------
  function updateGutter() {
    var n = code.value.split('\n').length, s = '';
    for (var i = 1; i <= n; i++) s += i + '\n';
    gutter.textContent = s;
    gutter.scrollTop = code.scrollTop;
  }
  function insertIndent(ta) {
    var indent = /\.rs$/.test(state.current ? state.current.file : '') ? '    ' : '  ';
    var s = ta.selectionStart, e = ta.selectionEnd;
    ta.value = ta.value.slice(0, s) + indent + ta.value.slice(e);
    ta.selectionStart = ta.selectionEnd = s + indent.length;
  }
  code.addEventListener('scroll', function () { gutter.scrollTop = code.scrollTop; });
  code.addEventListener('input', function () { updateGutter(); scheduleSave(); });
  code.addEventListener('keydown', function (e) {
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') { e.preventDefault(); run(); return; }
    if (e.key === 'Tab' && !e.shiftKey && !e.ctrlKey && !e.metaKey) { e.preventDefault(); insertIndent(code); updateGutter(); scheduleSave(); }
  });
  function scheduleSave() {
    if (!state.current) return;
    $('save-state').textContent = 'Editing…';
    clearTimeout(saveTimer);
    var slug = state.current.slug;
    saveTimer = setTimeout(function () {
      api('PUT', '/api/lessons/' + slug + '/code', code.value)
        .then(function () { if (state.current && state.current.slug === slug) $('save-state').textContent = 'Saved'; })
        .catch(function (e) { $('save-state').textContent = 'Not saved: ' + e.message; });
    }, 600);
  }
  $('reset-btn').addEventListener('click', function () {
    var b = $('reset-btn');
    if (!state.current) return;
    if (b.getAttribute('data-armed') !== '1') {
      b.setAttribute('data-armed', '1'); b.textContent = 'Click again to clear your code';
      setTimeout(function () { b.removeAttribute('data-armed'); b.textContent = 'Start over'; }, 3000);
      return;
    }
    b.removeAttribute('data-armed'); b.textContent = 'Start over';
    api('DELETE', '/api/lessons/' + state.current.slug + '/code').then(function (d) {
      code.value = d.code; updateGutter(); $('save-state').textContent = 'Starter code';
      $('results').innerHTML = ''; $('after').innerHTML = ''; renderHelp(false);
      if (d.stats) renderStatus(d.stats);
      if (d.next) state.next = d.next;
    });
  });

  // ---------- run & feedback ----------
  function run() {
    var btn = $('run-btn');
    if (!state.current || btn.disabled) return;
    btn.disabled = true; btn.textContent = 'Checking…';
    $('results').innerHTML = '<p class="muted">Running the checks…</p>';
    $('after').innerHTML = '';
    clearTimeout(saveTimer);
    api('POST', '/api/lessons/' + state.current.slug + '/run', code.value)
      .then(function (r) {
        $('save-state').textContent = 'Saved';
        if (r.next) state.next = r.next;
        renderResult(r);
        if (r.stats) renderStatus(r.stats);
        refresh().catch(function () {});
      })
      .catch(function (e) { $('results').innerHTML = '<div class="result bad"><b>Couldn’t run the checks.</b> ' + esc(e.message) + '</div>'; })
      .then(function () { btn.disabled = false; btn.textContent = '▶ Run & check'; });
  }
  $('run-btn').addEventListener('click', run);

  function renderResult(r) {
    var ok = r.total > 0 && r.failed === 0 && !r.timedOut && !r.compileErrors.length;
    var html = [];
    if (r.timedOut) html.push('<div class="result bad"><b>Stopped: your code ran for too long.</b> Look for a loop that never ends.</div>');
    else if (r.compileErrors.length) {
      html.push('<div class="result bad"><b>Your code doesn’t ' + (/\.rs$/.test(state.current.file) ? 'compile' : 'load') + ' yet.</b> Fix this first:</div>');
      r.compileErrors.forEach(function (e) {
        html.push('<div class="diag">' + (e.code ? '<span class="errcode">' + esc(e.code) + '</span> ' : '') + '<b>' + esc(e.title) + '</b><pre>' + esc(e.message) + '</pre></div>');
      });
    } else if (!r.total) html.push('<div class="result bad"><b>No checks ran.</b> Is the file complete?</div>');
    else {
      html.push('<div class="result ' + (ok ? 'ok' : 'bad') + '"><b>' + (ok ? '✓ All ' + r.total + ' checks pass.' : r.passed + ' of ' + r.total + ' checks pass.') + '</b>' +
        '<span class="meter"><span style="width:' + Math.round(100 * r.passed / r.total) + '%"></span></span></div>');
      html.push('<ul class="checks">' + r.tests.map(function (t) {
        if (t.ok) return '<li class="ok"><span class="mark">✓</span><span>' + esc(t.name) + '</span></li>';
        if (/not yet implemented|Error: TODO:/.test(t.message || '')) {
          var stub = /\.rs$/.test(state.current.file) ? 'todo!()' : 'its TODO';
          return '<li class="bad todo"><span class="mark">○</span><span>' + esc(t.name) + '<small>Not written yet: this function still has <code>' + stub + '</code>.</small></span></li>';
        }
        var detail = t.expected != null || t.actual != null
          ? '<div class="diff"><span>Expected</span><code>' + esc(t.expected) + '</code><span>Got</span><code>' + esc(t.actual) + '</code></div>'
          : (t.message ? '<pre>' + esc(t.message.split('\n').slice(0, 8).join('\n')) + '</pre>' : '');
        return '<li class="bad"><span class="mark">✗</span><span>' + esc(t.name) + detail + '</span></li>';
      }).join('') + '</ul>');
    }
    if (r.output) html.push('<details class="raw"><summary>Full output</summary><pre>' + esc(r.output) + '</pre></details>');
    $('results').innerHTML = html.join('');
    if (ok) renderSuccess(r); else renderHelp(true);
  }

  function renderSuccess(r) {
    $('help').innerHTML = '';
    var st = r.stats || {};
    var first = st.first_pass_run === st.runs;
    var html = '<div class="success"><div class="success-head"><span class="big-check">✓</span><div><b>' +
      (first ? 'Solved' + (st.runs === 1 ? ' on the first try!' : ' in ' + plural(st.runs, 'run') + '.') : 'Solved again.') + '</b>' +
      '<span>How did that feel? It shapes what comes next.</span></div></div>' +
      '<div class="rate" id="rate"><button type="button" data-rate="easy">Too easy</button><button type="button" data-rate="right">About right</button><button type="button" data-rate="hard">Too hard</button></div>' +
      '<div id="next-slot"></div>' +
      (state.review ? '<div class="review-tip">↻ ' + md(state.review).replace(/^<p>|<\/p>$/g, '') + '</div>' : '') +
      (state.tutor.enabled ? '<button type="button" class="link" id="drill-btn">Want more practice on this? Get a new exercise made for you →</button>' : '') + '</div>';
    $('after').innerHTML = html;
    showNext(r.next);
    wireDrillButton();
    $('rate').addEventListener('click', function (e) {
      var d = e.target.getAttribute('data-rate');
      if (!d || !state.current) return;
      api('POST', '/api/lessons/' + state.current.slug + '/feedback', d).then(function (res) {
        $('rate').innerHTML = '<span class="muted">Noted: ' + esc(e.target.textContent.toLowerCase()) + '. Your next step has adapted.</span>';
        state.next = res.next;
        showNext(res.next, true);
        refresh().catch(function () {});
      });
    });
    $('after').scrollIntoView({ behavior: 'smooth', block: 'nearest' });
  }

  function showNext(next, changed) {
    var slot = $('next-slot');
    if (!slot || !next) return;
    if (next.kind === 'done' || !next.slug) { slot.innerHTML = '<p class="muted">' + esc(next.reason) + '.</p>'; return; }
    if (state.current && next.slug === state.current.slug) { slot.innerHTML = ''; return; }
    slot.innerHTML = '<a class="next-btn' + (changed ? ' pulse' : '') + '" href="#learn/' + esc(next.slug) + '"><span><small>Next · ' + esc(KIND[next.kind] || 'Lesson') + '</small>' +
      esc(next.title) + '<em>' + esc(next.reason) + '</em></span><span class="arrow">→</span></a>';
  }

  // ---------- help: hint ladder, then the tutor ----------
  function renderHelp(afterFail) {
    var slug = state.current ? state.current.slug : '';
    var shown = load('hints.' + slug, 0), hints = state.hints || [];
    var html = '';
    if (hints.length || state.tutor.enabled) {
      html += '<div class="help' + (afterFail ? ' open' : '') + '"><p class="help-title">' + (afterFail ? 'Stuck? Get a nudge, then try again.' : 'Need a nudge?') + '</p>';
      if (hints.length) {
        html += '<ol class="hints">' + hints.slice(0, shown).map(function (h) { return '<li>' + md(h).replace(/^<p>|<\/p>$/g, '') + '</li>'; }).join('') + '</ol>';
        if (shown < hints.length) html += '<button type="button" class="btn" id="hint-more">Show hint ' + (shown + 1) + ' of ' + hints.length + '</button>';
      }
      if (state.tutor.enabled && afterFail) {
        html += '<button type="button" class="btn trace-btn" id="trace-btn">▶ Show me what my code does</button><div id="trace-out"></div>';
      }
      if (state.tutor.enabled) {
        html += '<div class="ask"><input type="text" id="ask-q" placeholder="Or ask the tutor about your code…" aria-label="Question for the tutor">' +
          '<button type="button" class="btn" id="ask-btn">Ask</button></div><div id="ask-out"></div>' +
          '<button type="button" class="link" id="drill-btn">Want more practice on this? Get a new exercise made for you →</button>';
      } else if (state.tutor.reason) {
        html += '<p class="muted small">The AI tutor is off: ' + esc(state.tutor.reason) + '</p>';
      }
      html += '<p class="muted small">Try on your own first. Your history notes when you use hints.</p></div>';
    }
    $('help').innerHTML = html;
    var more = $('hint-more');
    if (more) more.addEventListener('click', function () {
      store('hints.' + slug, shown + 1);
      api('POST', '/api/lessons/' + slug + '/log', { kind: 'hint', payload: { source: 'lesson', level: shown + 1, question: null } }, true).catch(function () {});
      renderHelp(afterFail);
    });
    var trace = $('trace-btn');
    if (trace) trace.addEventListener('click', function () {
      trace.disabled = true;
      $('trace-out').innerHTML = '<p class="muted">Tracing your code step by step, then checking the trace against your real code (about half a minute)…</p>';
      api('POST', '/api/lessons/' + slug + '/visualize', { code: code.value }, true).then(function (d) {
        var v = d.verified || {};
        $('trace-out').innerHTML = '<div class="trace"><div class="visual" data-spec="' + esc(encodeURIComponent(JSON.stringify(d.visual))) + '"></div>' +
          '<p class="muted small">✓ Checked on your real code: ' + (v.call ? '<code>' + esc(v.call) + '</code> really gives <code>' + esc(v.prints) + '</code>.' : 'it really fails with <code>' + esc(v.prints) + '</code>.') + '</p></div>';
        if (window.hydrateVisuals) window.hydrateVisuals($('trace-out'));
        trace.textContent = '↻ Trace it again';
      }).catch(function (e) { $('trace-out').innerHTML = '<p class="bad-text">' + esc(e.message) + '</p>'; })
        .then(function () { trace.disabled = false; });
    });
    var ask = $('ask-btn');
    if (ask) {
      var doAsk = function () {
        ask.disabled = true; $('ask-out').innerHTML = '<p class="muted">Reading your code…</p>';
        api('POST', '/api/lessons/' + slug + '/hint', { code: code.value, question: $('ask-q').value }, true)
          .then(function (d) { $('ask-out').innerHTML = '<div class="tutor-reply">' + md(d.hint) + '</div>'; })
          .catch(function (e) { $('ask-out').innerHTML = '<p class="bad-text">' + esc(e.message) + '</p>'; })
          .then(function () { ask.disabled = false; });
      };
      ask.addEventListener('click', doAsk);
      $('ask-q').addEventListener('keydown', function (e) { if (e.key === 'Enter') doAsk(); });
      wireDrillButton();
    }
  }

  // "Get a new exercise made for you": the tutor writes and verifies a drill for this lesson.
  function wireDrillButton() {
    var b = $('drill-btn');
    if (!b || !state.current) return;
    var slug = state.current.slug;
    b.addEventListener('click', function () {
      b.disabled = true; b.textContent = 'Writing an exercise for you and checking it against a working solution (about a minute)…';
      api('POST', '/api/lessons/' + slug + '/drill', '{}').then(function (d) {
        return refresh().then(function () {
          renderOutline();
          b.outerHTML = '<a class="next-btn" href="#learn/' + esc(d.slug) + '"><span><small>New · made for you</small>' + esc(d.title) + '<em>' + esc(d.targets) + '</em></span><span class="arrow">→</span></a>';
        });
      }).catch(function (e) { b.disabled = false; b.textContent = 'Couldn’t make one: ' + e.message + ' Try again?'; });
    });
  }

  // ---------- History ----------
  function describe(e) {
    var l = lessonById(e.lesson_id), t = l ? l.title : (e.lesson_id || '');
    var p = e.payload || {};
    switch (e.kind) {
      case 'pass': return ['ok', 'Solved “' + t + '”'];
      case 'fail': return ['bad', (p.errors && p.errors.length ? 'Didn’t compile (' + p.errors.slice(0, 2).join(', ') + ')' : p.passed + ' of ' + p.total + ' checks passed') + ' on “' + t + '”'];
      case 'feedback': return ['info', 'Rated “' + t + '” ' + ({ easy: 'too easy', right: 'about right', hard: 'too hard' }[p.difficulty] || p.difficulty)];
      case 'hint': return ['info', p.source === 'lesson' ? 'Used hint ' + p.level + ' on “' + t + '”' : p.source === 'visual' ? 'Watched the tutor trace your code on “' + t + '”' : 'Asked the tutor about “' + t + '”' + (p.question ? ': ' + p.question : '')];
      case 'predict': return ['info', 'Quick check on “' + t + '”: ' + (p.correct ? 'right' : 'not quite')];
      case 'generated': return ['new', 'The tutor made a new exercise: “' + t + '”'];
      case 'reset': return ['info', 'Started over on “' + t + '”'];
      default: return null;
    }
  }
  function renderHistory() {
    var groups = [], last = null;
    state.history.forEach(function (e) {
      var d = describe(e);
      if (!d) return;
      var k = dayKey(e.ts);
      if (!last || last.key !== k) { last = { key: k, date: new Date(e.ts), items: [] }; groups.push(last); }
      last.items.push({ ts: e.ts, cls: d[0], text: d[1] });
    });
    $('history-list').innerHTML = groups.length ? groups.map(function (g) {
      return '<section class="day"><h3>' + esc(g.date.toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric' })) + '</h3><ul class="timeline">' +
        g.items.map(function (i) {
          return '<li class="' + i.cls + '"><time>' + esc(new Date(i.ts).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })) + '</time><span>' + esc(i.text) + '</span></li>';
        }).join('') + '</ul></section>';
    }).join('') : '<p class="muted">Nothing yet. Your first run will show up here.</p>';
  }

  // ---------- Profile ----------
  function renderProfile() {
    var p = state.profile;
    if (!p) return;
    var f = p.pace.feedback, s = p.sessions;
    var rows = p.lessons.map(function (l) {
      var review = l.due_now ? '<span class="chip">due now</span>' : l.due ? new Date(l.due).toLocaleDateString(undefined, { month: 'short', day: 'numeric' }) : '–';
      return '<tr><td>' + esc(l.title) + (l.kind !== 'core' ? ' <span class="muted small">(' + (l.kind === 'drill' ? 'practice' : 'challenge') + ')</span>' : '') + '</td><td>' + esc(l.status) +
        '</td><td>' + l.runs + '</td><td>' + (l.feedback ? esc({ easy: 'too easy', right: 'about right', hard: 'too hard' }[l.feedback]) : '–') + '</td><td>' + review + '</td></tr>';
    }).join('');
    $('profile-body').innerHTML =
      '<div class="cards3">' +
      '<div class="card"><h3>Pace</h3><p class="big">' + (p.pace.median_runs_to_solve === null ? '–' : p.pace.median_runs_to_solve) + '</p><p class="muted small">runs to solve a lesson (median)</p></div>' +
      '<div class="card"><h3>How lessons felt</h3><p class="big">' + f.easy + ' · ' + f.right + ' · ' + f.hard + '</p><p class="muted small">too easy · about right · too hard</p></div>' +
      '<div class="card"><h3>Practice</h3><p class="big">' + s.minutes_total + ' min</p><p class="muted small">' + plural(s.count, 'session') + ', ' + plural(s.streak_days, 'day') + ' streak</p></div>' +
      '</div>' +
      '<div class="card">' + conceptBars(0, 'How well you know each idea') + '</div>' +
      '<div class="card"><h3>Mistakes that keep coming back</h3>' + (p.recurring.length ? '<ul class="chips">' + p.recurring.map(function (r) {
        return '<li><code>' + esc(r.what) + '</code> ×' + r.count + '</li>';
      }).join('') + '</ul><p class="muted small">The tutor aims new practice at these.</p>' : '<p class="muted small">None yet.</p>') + '</div>' +
      '<div class="card"><h3>Every exercise</h3><div class="table-wrap"><table><thead><tr><th>Exercise</th><th>Status</th><th>Runs</th><th>Felt</th><th>Next review</th></tr></thead><tbody>' + rows + '</tbody></table></div></div>' +
      '<p class="muted small">This summary is also written to <code>PROGRESS.md</code> after every run. Commit it so Claude sessions can see your progress.</p>';
  }

  // ---------- start ----------
  api('GET', '/api/info').then(function (info) {
    state.info = info;
    state.tutor = info.tutor || state.tutor;
    info.trackName = info.track === 'rust' ? 'Rust' : 'Node.js';
    document.title = info.title;
    // "Rust Lab" → <strong>Rust</strong> Lab; "Node.js Basics" → <strong>Node.js</strong> Basics
    var words = info.title.split(' ');
    $('brand').innerHTML = '<strong>' + esc(words.slice(0, -1).join(' ') || info.title) + '</strong> ' + esc(words.length > 1 ? words[words.length - 1] : '');
    if (info.accent) document.documentElement.style.setProperty('--accent', info.accent);
    return refresh();
  }).then(route).catch(function (e) {
    document.querySelector('main').innerHTML = '<div class="card"><h2>Couldn’t reach the app server</h2><p>' + esc(e.message) + '</p><p>Is <code>npm start</code> / <code>cargo run</code> still running?</p></div>';
  });
})();

// Step-through visualizations for lessons (```visual and ```anatomy blocks; format in
// docs/SHARED-FORMAT.md, section 1). Shared byte-for-byte by the learn-* repos. No dependencies.
//
// A visual has scenarios; a scenario has code, regions (columns) and steps. Each step
// changes the scene: boxes are set/updated by id (`set`), removed (`drop`), arrows are
// replaced (`arrows`), and a code line is highlighted. Moving a box to another region
// animates it there.
(function () {
  'use strict';
  var NS = 'http://www.w3.org/2000/svg';
  var reduced = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  var uid = 0;

  function esc(s) {
    return String(s == null ? '' : s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }
  function inline(text) {
    var html = window.renderMarkdown ? window.renderMarkdown(String(text || '')) : esc(text);
    return html.replace(/^<p>|<\/p>$/g, '');
  }

  /** The scene after `index` steps: boxes (ordered), arrows, and the step itself. */
  function sceneAt(scenario, index) {
    var boxes = [], arrows = [];
    for (var i = 0; i <= index; i++) {
      var step = scenario.steps[i];
      (step.drop || []).forEach(function (id) { boxes = boxes.filter(function (b) { return b.id !== id; }); });
      (step.set || []).forEach(function (s) {
        var found = boxes.filter(function (b) { return b.id === s.id; })[0];
        if (found) {
          for (var k in s) found[k] = s[k];
          if (s['in'] && s['in'] !== found._region) { boxes.splice(boxes.indexOf(found), 1); boxes.push(found); }
          found._region = found['in'];
        } else {
          var b = {}; for (var k2 in s) b[k2] = s[k2];
          b._region = b['in'];
          boxes.push(b);
        }
      });
      if (step.arrows) arrows = step.arrows;
    }
    return { boxes: boxes.map(function (b) { var c = {}; for (var k in b) c[k] = b[k]; return c; }), arrows: arrows, step: scenario.steps[index] };
  }

  function build(el, spec) {
    var id = 'v' + (++uid);
    var groups = [];
    spec.scenarios.forEach(function (sc, i) {
      var g = sc.group || 'How it works';
      var found = groups.filter(function (x) { return x.name === g; })[0];
      if (!found) groups.push(found = { name: g, items: [] });
      found.items.push(i);
    });
    el.innerHTML =
      '<div class="vz-head">' + (spec.title ? '<h3>' + esc(spec.title) + '</h3>' : '') +
      (spec.intro ? '<p>' + inline(spec.intro) + '</p>' : '') + '</div>' +
      '<div class="vz-scenarios">' + groups.map(function (g) {
        return '<div class="vz-group"><span class="vz-group-name">' + esc(g.name) + '</span>' + g.items.map(function (i) {
          return '<button type="button" class="vz-sc' + (/wrong/i.test(g.name) ? ' vz-sc-wrong' : '') + '" data-sc="' + i + '">' + esc(spec.scenarios[i].name) + '</button>';
        }).join('') + '</div>';
      }).join('') + '</div>' +
      '<div class="vz-stage"><div class="vz-regions"></div><svg class="vz-arrows" aria-hidden="true"><defs>' +
      ['ok', 'bad', 'muted', 'accent'].map(function (k) {
        return '<marker id="' + id + '-' + k + '" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" class="vz-head-' + k + '"/></marker>';
      }).join('') + '</defs></svg></div>' +
      '<div class="vz-bottom"><ol class="vz-code"></ol><div class="vz-explain"><div class="vz-err" hidden></div>' +
      '<p class="vz-k">Step</p><p class="vz-say" aria-live="polite"></p><p class="vz-k vz-rule-k">Rule</p><p class="vz-rule"></p></div></div>' +
      '<div class="vz-controls"><button type="button" class="vz-btn" data-act="back">← Back</button>' +
      '<button type="button" class="vz-btn vz-play" data-act="play">▶ Play</button>' +
      '<button type="button" class="vz-btn" data-act="next">Next →</button><span class="vz-pills"></span></div>';
    el.tabIndex = 0;

    var state = { sc: 0, step: 0, timer: null };
    var regionsEl = el.querySelector('.vz-regions'), svg = el.querySelector('.vz-arrows'), stage = el.querySelector('.vz-stage');

    function setScenario(i) {
      stop();
      state.sc = i; state.step = 0;
      var sc = spec.scenarios[i];
      Array.prototype.forEach.call(el.querySelectorAll('.vz-sc'), function (b) { b.classList.toggle('on', +b.getAttribute('data-sc') === i); });
      var regions = sc.regions || spec.regions || [];
      regionsEl.style.gridTemplateColumns = 'repeat(' + regions.length + ', minmax(0, 1fr))';
      regionsEl.innerHTML = regions.map(function (r) {
        return '<div class="vz-region' + (r.kind === 'output' ? ' vz-output' : '') + '" data-region="' + esc(r.id) + '"><p class="vz-region-label">' + esc(r.label) + '</p><div class="vz-slots"></div></div>';
      }).join('');
      var code = String(sc.code || '').replace(/\n$/, '').split('\n');
      el.querySelector('.vz-code').innerHTML = code.map(function (line, n) {
        return '<li data-line="' + (n + 1) + '"><span class="vz-ln">' + (n + 1) + '</span><code>' + esc(line) + '</code></li>';
      }).join('');
      el.querySelector('.vz-code').hidden = !sc.code;
      el.querySelector('.vz-pills').innerHTML = sc.steps.map(function (s, n) {
        return '<button type="button" class="vz-pill" data-step="' + n + '" aria-label="Step ' + (n + 1) + '">' + (n + 1) + '</button>';
      }).join('');
      render(false);
    }

    function render(animate) {
      var sc = spec.scenarios[state.sc], scene = sceneAt(sc, state.step), step = scene.step;
      // FLIP: remember where boxes were, update the DOM, then animate from the old place.
      var before = {};
      Array.prototype.forEach.call(regionsEl.querySelectorAll('.vz-box'), function (b) { before[b.getAttribute('data-id')] = b.getBoundingClientRect(); });
      var existing = {};
      Array.prototype.forEach.call(regionsEl.querySelectorAll('.vz-box'), function (b) { existing[b.getAttribute('data-id')] = b; });
      var keep = {};
      scene.boxes.forEach(function (b) {
        var slot = regionsEl.querySelector('[data-region="' + b._region + '"] .vz-slots');
        if (!slot) return;
        var node = existing[b.id];
        var isNew = !node;
        if (isNew) { node = document.createElement('div'); node.setAttribute('data-id', b.id); }
        node.className = 'vz-box vz-' + (b.state || 'normal') + (isNew && animate && !reduced ? ' vz-enter' : '');
        node.innerHTML = '<div class="vz-box-top"><span class="vz-title">' + esc(b.title != null ? b.title : b.id) + '</span>' +
          (b.tag ? '<span class="vz-tag">' + esc(b.tag) + '</span>' : '') + '</div>' +
          (b.value != null ? '<div class="vz-value">' + esc(b.value) + '</div>' : '') +
          (b.sub ? '<div class="vz-sub">' + inline(b.sub) + '</div>' : '');
        slot.appendChild(node);
        keep[b.id] = 1;
      });
      Object.keys(existing).forEach(function (k) {
        if (keep[k]) return;
        var gone = existing[k];
        if (!animate || reduced) { gone.remove(); return; }
        gone.classList.add('vz-leave');
        setTimeout(function () { gone.remove(); drawArrows(scene.arrows); }, 260);
      });
      if (animate && !reduced) {
        Array.prototype.forEach.call(regionsEl.querySelectorAll('.vz-box'), function (b) {
          var old = before[b.getAttribute('data-id')];
          if (!old) return;
          var now = b.getBoundingClientRect(), dx = old.left - now.left, dy = old.top - now.top;
          if (!dx && !dy) return;
          b.style.transition = 'none';
          b.style.transform = 'translate(' + dx + 'px,' + dy + 'px)';
          requestAnimationFrame(function () {
            requestAnimationFrame(function () { b.style.transition = ''; b.style.transform = ''; });
          });
        });
      }
      // Code line, explanation, rule, error.
      var lines = step.line == null ? [] : [].concat(step.line);
      Array.prototype.forEach.call(el.querySelectorAll('.vz-code li'), function (li) {
        var n = +li.getAttribute('data-line');
        li.classList.toggle('on', lines.indexOf(n) >= 0);
        li.classList.toggle('bad', !!step.error && lines.indexOf(n) >= 0);
      });
      el.querySelector('.vz-say').innerHTML = inline(step.say || '');
      var rule = step.rule || '';
      el.querySelector('.vz-rule').innerHTML = inline(rule);
      el.querySelector('.vz-rule').hidden = el.querySelector('.vz-rule-k').hidden = !rule;
      var err = el.querySelector('.vz-err');
      err.hidden = !step.error;
      err.innerHTML = step.error ? '✗ ' + inline(step.error) : '';
      Array.prototype.forEach.call(el.querySelectorAll('.vz-pill'), function (p) {
        var n = +p.getAttribute('data-step');
        p.classList.toggle('on', n === state.step);
        p.classList.toggle('seen', n < state.step);
      });
      el.querySelector('[data-act="back"]').disabled = state.step === 0;
      el.querySelector('[data-act="next"]').disabled = state.step === sc.steps.length - 1;
      // Arrows after layout (and again once moves have settled).
      requestAnimationFrame(function () { drawArrows(scene.arrows, animate); });
      if (animate && !reduced) setTimeout(function () { drawArrows(scene.arrows, false); }, 420);
    }

    function rectOf(boxId) {
      var b = regionsEl.querySelector('.vz-box[data-id="' + boxId + '"]:not(.vz-leave)');
      if (!b) return null;
      var s = stage.getBoundingClientRect();
      var x = 0, y = 0, n = b;
      while (n && n !== stage) { x += n.offsetLeft; y += n.offsetTop; n = n.offsetParent; }
      if (n !== stage) { var r = b.getBoundingClientRect(); x = r.left - s.left; y = r.top - s.top; }
      return { left: x, top: y, right: x + b.offsetWidth, bottom: y + b.offsetHeight, cy: y + b.offsetHeight / 2, cx: x + b.offsetWidth / 2 };
    }

    function drawArrows(arrows, animate) {
      Array.prototype.forEach.call(svg.querySelectorAll('.vz-arrow, .vz-alabel'), function (p) { p.remove(); });
      svg.setAttribute('width', stage.clientWidth);
      svg.setAttribute('height', stage.clientHeight);
      (arrows || []).forEach(function (a) {
        var f = rectOf(a.from), t = rectOf(a.to);
        if (!f || !t) return;
        var d, x1, y1, x2, y2;
        if (t.left >= f.right) { x1 = f.right; y1 = f.cy; x2 = t.left - 2; y2 = t.cy; var c = Math.max(40, (x2 - x1) / 2); d = 'M' + x1 + ',' + y1 + ' C' + (x1 + c) + ',' + y1 + ' ' + (x2 - c) + ',' + y2 + ' ' + x2 + ',' + y2; }
        else if (t.right <= f.left) { x1 = f.left; y1 = f.cy; x2 = t.right + 2; y2 = t.cy; var c2 = Math.max(40, (x1 - x2) / 2); d = 'M' + x1 + ',' + y1 + ' C' + (x1 - c2) + ',' + y1 + ' ' + (x2 + c2) + ',' + y2 + ' ' + x2 + ',' + y2; }
        else if (t.top >= f.bottom - 2 || f.top >= t.bottom - 2) {
          // Stacked regions (narrow screens): go from one box's edge straight to the other's.
          var down = t.top >= f.bottom - 2;
          x1 = f.cx; y1 = down ? f.bottom : f.top; x2 = t.cx; y2 = down ? t.top - 2 : t.bottom + 2;
          var cv = Math.max(24, Math.abs(y2 - y1) / 2) * (down ? 1 : -1);
          d = 'M' + x1 + ',' + y1 + ' C' + x1 + ',' + (y1 + cv) + ' ' + x2 + ',' + (y2 - cv) + ' ' + x2 + ',' + y2;
        }
        else { x1 = f.right; y1 = f.cy; x2 = t.right + 2; y2 = t.cy; var out = Math.max(f.right, t.right) + 46; d = 'M' + x1 + ',' + y1 + ' C' + out + ',' + y1 + ' ' + out + ',' + y2 + ' ' + x2 + ',' + y2; }
        var kind = a.state === 'bad' ? 'bad' : a.state === 'muted' ? 'muted' : a.state === 'accent' ? 'accent' : 'ok';
        var path = document.createElementNS(NS, 'path');
        path.setAttribute('d', d);
        path.setAttribute('class', 'vz-arrow vz-arrow-' + kind + (a.style === 'solid' ? '' : ' vz-dashed'));
        path.setAttribute('marker-end', 'url(#' + id + '-' + kind + ')');
        svg.appendChild(path);
        if (animate && !reduced) {
          var len = path.getTotalLength();
          path.style.strokeDasharray = len; path.style.strokeDashoffset = len;
          path.getBoundingClientRect();
          path.style.transition = 'stroke-dashoffset .5s ease';
          path.style.strokeDashoffset = '0';
          setTimeout(function () { path.style.strokeDasharray = ''; path.style.strokeDashoffset = ''; path.style.transition = ''; }, 520);
        }
        if (a.label) {
          var tx = document.createElementNS(NS, 'text');
          tx.setAttribute('x', (x1 + x2) / 2); tx.setAttribute('y', (y1 + y2) / 2 - 8);
          tx.setAttribute('class', 'vz-alabel vz-alabel-' + kind);
          tx.setAttribute('text-anchor', 'middle');
          tx.textContent = a.label;
          svg.appendChild(tx);
        }
      });
    }

    function go(n) {
      var sc = spec.scenarios[state.sc];
      if (n < 0 || n >= sc.steps.length || n === state.step) return;
      state.step = n;
      render(true);
    }
    function stop() {
      if (state.timer) { clearInterval(state.timer); state.timer = null; }
      var p = el.querySelector('.vz-play'); if (p) p.textContent = '▶ Play';
    }
    function play() {
      if (state.timer) { stop(); return; }
      var sc = spec.scenarios[state.sc];
      if (state.step >= sc.steps.length - 1) { state.step = 0; render(true); }
      el.querySelector('.vz-play').textContent = '❚❚ Pause';
      state.timer = setInterval(function () {
        if (state.step >= spec.scenarios[state.sc].steps.length - 1) { stop(); return; }
        go(state.step + 1);
      }, 2400);
    }

    el.addEventListener('click', function (e) {
      var t = e.target.closest('button');
      if (!t || !el.contains(t)) return;
      if (t.hasAttribute('data-sc')) setScenario(+t.getAttribute('data-sc'));
      else if (t.hasAttribute('data-step')) { stop(); go(+t.getAttribute('data-step')); }
      else if (t.getAttribute('data-act') === 'next') { stop(); go(state.step + 1); }
      else if (t.getAttribute('data-act') === 'back') { stop(); go(state.step - 1); }
      else if (t.getAttribute('data-act') === 'play') play();
    });
    el.addEventListener('keydown', function (e) {
      if (e.target.closest('textarea, input')) return;
      if (e.key === 'ArrowRight') { e.preventDefault(); stop(); go(state.step + 1); }
      if (e.key === 'ArrowLeft') { e.preventDefault(); stop(); go(state.step - 1); }
    });
    var resizeTimer = null;
    window.addEventListener('resize', function () {
      clearTimeout(resizeTimer);
      resizeTimer = setTimeout(function () { if (document.body.contains(el)) drawArrows(sceneAt(spec.scenarios[state.sc], state.step).arrows, false); }, 100);
    });
    setScenario(0);
  }

  // ---------- anatomy: one piece of code, token by token ----------

  /** Where each part sits in the code: parts are found in order; text between them stays plain. */
  function anatomySpans(spec) {
    var code = String(spec.code || ''), at = 0;
    return (spec.parts || []).map(function (p) {
      var i = code.indexOf(p.text, at);
      if (i < 0 || !p.text) return null;
      at = i + p.text.length;
      return { start: i, end: at };
    });
  }

  function buildAnatomy(el, spec) {
    var code = String(spec.code || ''), parts = spec.parts || [], spans = anatomySpans(spec);
    var html = '', at = 0;
    parts.forEach(function (p, i) {
      var s = spans[i];
      if (!s) return;
      html += esc(code.slice(at, s.start)) + '<button type="button" class="an-tok an-' + esc(p.kind || 'symbol') + '" data-i="' + i + '">' + esc(p.text) + '</button>';
      at = s.end;
    });
    html += esc(code.slice(at));
    el.innerHTML =
      (spec.title ? '<div class="vz-head"><h3>' + esc(spec.title) + '</h3>' + (spec.intro ? '<p>' + inline(spec.intro) + '</p>' : '') + '</div>' : '') +
      '<pre class="an-code">' + html + '</pre>' +
      '<div class="an-detail" aria-live="polite"></div>' +
      '<div class="vz-controls"><button type="button" class="vz-btn" data-act="prev">← Back</button>' +
      '<button type="button" class="vz-btn" data-act="next">Next →</button><span class="an-count"></span></div>';
    el.tabIndex = 0;
    var cur = 0, toks = el.querySelectorAll('.an-tok');
    function show(i) {
      cur = Math.max(0, Math.min(parts.length - 1, i));
      var p = parts[cur];
      Array.prototype.forEach.call(toks, function (t) { t.classList.toggle('on', +t.getAttribute('data-i') === cur); });
      el.querySelector('.an-detail').innerHTML =
        '<div class="an-what"><code class="an-big an-' + esc(p.kind || 'symbol') + '">' + esc(p.text) + '</code><span class="an-kind">' + esc(p.kind || 'symbol') + '</span></div>' +
        '<p class="an-say">' + inline(p.say) + '</p>' +
        (p.java ? '<p class="an-java"><b>Java:</b> ' + inline(p.java) + '</p>' : '');
      el.querySelector('[data-act="prev"]').disabled = cur === 0;
      el.querySelector('[data-act="next"]').disabled = cur === parts.length - 1;
      el.querySelector('.an-count').textContent = (cur + 1) + ' / ' + parts.length;
    }
    el.addEventListener('click', function (e) {
      var t = e.target.closest('[data-i], [data-act]');
      if (!t) return;
      if (t.hasAttribute('data-i')) show(+t.getAttribute('data-i'));
      else show(cur + (t.getAttribute('data-act') === 'next' ? 1 : -1));
    });
    el.addEventListener('keydown', function (e) {
      if (e.key === 'ArrowRight') { show(cur + 1); e.preventDefault(); }
      if (e.key === 'ArrowLeft') { show(cur - 1); e.preventDefault(); }
    });
    if (parts.length) show(0);
  }

  /** Turn every .visual and .anatomy placeholder inside `root` into a live visualization. */
  window.hydrateVisuals = function (root) {
    Array.prototype.forEach.call(root.querySelectorAll('.anatomy[data-spec]'), function (el) {
      var spec;
      try { spec = JSON.parse(decodeURIComponent(el.getAttribute('data-spec'))); }
      catch (e) { el.innerHTML = '<p class="vz-broken">This anatomy could not be read: ' + esc(e.message) + '</p>'; return; }
      el.removeAttribute('data-spec');
      buildAnatomy(el, spec);
    });
    Array.prototype.forEach.call(root.querySelectorAll('.visual[data-spec]'), function (el) {
      var spec;
      try { spec = JSON.parse(decodeURIComponent(el.getAttribute('data-spec'))); }
      catch (e) { el.innerHTML = '<p class="vz-broken">This visualization could not be read: ' + esc(e.message) + '</p>'; return; }
      el.removeAttribute('data-spec');
      build(el, spec);
    });
  };
  window.visualSceneAt = sceneAt; // for tests and debugging
  window.anatomySpans = anatomySpans;
})();

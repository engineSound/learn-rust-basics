// Minimal Markdown renderer for lesson.md files (shared by both apps).
// Supports headings, paragraphs, fenced code, tables, lists, inline code,
// bold, italic and links, plus two lesson blocks: ```run (a runnable example)
// ```predict (a quick-check question) and ```visual (a step-through animation),
// which app.js and visual.js turn into widgets. Lessons are written by us, but text is still escaped.
(function () {
  function esc(s) {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }
  function inline(s) {
    // Protect code spans first, so bold/italic can wrap them: **Use `===`.**
    var codes = [];
    var text = esc(s.replace(/`([^`]*)`/g, function (_, c) { codes.push(c); return '\u0000' + (codes.length - 1) + '\u0000'; }));
    text = text
      .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
      .replace(/(^|[^*])\*([^*\s][^*]*)\*/g, '$1<em>$2</em>')
      .replace(/\[([^\]]+)\]\((https?:[^)\s]+)\)/g, '<a href="$2" target="_blank" rel="noopener">$1</a>');
    return text.replace(/\u0000(\d+)\u0000/g, function (_, i) { return '<code>' + esc(codes[+i]) + '</code>'; });
  }
  function cells(line) {
    return line.trim().replace(/^\|/, '').replace(/\|$/, '').split('|').map(function (c) { return c.trim(); });
  }
  function render(md) {
    var lines = md.replace(/\r\n/g, '\n').split('\n');
    var out = [], i = 0;
    while (i < lines.length) {
      var line = lines[i];
      if (/^```/.test(line)) {
        var lang = line.slice(3).trim().toLowerCase();
        var buf = []; i++;
        while (i < lines.length && !/^```/.test(lines[i])) buf.push(lines[i++]);
        i++;
        var text = buf.join('\n');
        if (lang === 'visual') {
          // A step-through visualization (JSON); visual.js turns it into an animated scene.
          out.push('<div class="visual" data-spec="' + esc(encodeURIComponent(text)) + '"></div>');
        } else if (lang === 'anatomy') {
          // One piece of code taken apart token by token (JSON); visual.js makes it explorable.
          out.push('<div class="anatomy" data-spec="' + esc(encodeURIComponent(text)) + '"></div>');
        } else if (lang === 'run') {
          // An editable example the learner can run; app.js turns it into a widget.
          out.push('<div class="try" data-code="' + esc(encodeURIComponent(text)) + '"></div>');
        } else if (lang === 'predict') {
          // Code, then "---", then "options: a | b | c", "answer: b", "why: ...".
          var parts = text.split(/^---\s*$/m), meta = {};
          (parts[1] || '').split('\n').forEach(function (l) {
            var m = /^(\w+):\s*(.*)$/.exec(l.trim());
            if (m) meta[m[1]] = m[2];
          });
          out.push('<div class="predict" data-code="' + esc(encodeURIComponent(parts[0].replace(/\n+$/, ''))) +
            '" data-options="' + esc(encodeURIComponent(meta.options || '')) + '" data-answer="' + esc(encodeURIComponent(meta.answer || '')) +
            '" data-why="' + esc(encodeURIComponent(meta.why || '')) + '"></div>');
        } else {
          out.push('<pre><code>' + esc(text) + '</code></pre>');
        }
      } else if (/^#{1,6} /.test(line)) {
        var level = line.match(/^#+/)[0].length;
        out.push('<h' + level + '>' + inline(line.slice(level + 1)) + '</h' + level + '>');
        i++;
      } else if (/^\s*\|/.test(line) && i + 1 < lines.length && /^\s*\|?\s*:?-+/.test(lines[i + 1])) {
        var head = cells(line); i += 2;
        var rows = [];
        while (i < lines.length && /^\s*\|/.test(lines[i])) rows.push(cells(lines[i++]));
        out.push('<table><thead><tr>' + head.map(function (c) { return '<th>' + inline(c) + '</th>'; }).join('') +
          '</tr></thead><tbody>' + rows.map(function (r) {
            return '<tr>' + r.map(function (c) { return '<td>' + inline(c) + '</td>'; }).join('') + '</tr>';
          }).join('') + '</tbody></table>');
      } else if (/^\s*([-*]|\d+\.) /.test(line)) {
        var ordered = /^\s*\d+\./.test(line), items = [];
        while (i < lines.length && /^\s*([-*]|\d+\.) /.test(lines[i])) items.push(lines[i++].replace(/^\s*([-*]|\d+\.) /, ''));
        var tag = ordered ? 'ol' : 'ul';
        out.push('<' + tag + '>' + items.map(function (t) { return '<li>' + inline(t) + '</li>'; }).join('') + '</' + tag + '>');
      } else if (line.trim() === '') {
        i++;
      } else {
        var para = [];
        while (i < lines.length && lines[i].trim() !== '' && !/^(```|#{1,6} |\s*\||\s*([-*]|\d+\.) )/.test(lines[i])) para.push(lines[i++]);
        out.push('<p>' + inline(para.join(' ')) + '</p>');
      }
    }
    return out.join('\n');
  }
  window.renderMarkdown = render;
})();

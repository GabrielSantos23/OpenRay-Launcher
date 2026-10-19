(function () {
  'use strict';
  var extensionId = window.__FLOWKEY_EXTENSION_ID__ || '';
  var listeners = [];
  var webview = {
    postMessage: function (message) {
      window.parent.postMessage(message, '*');
    },
    addEventListener: function (type, handler) {
      if (type === 'message') listeners.push(handler);
    },
    removeEventListener: function (type, handler) {
      var index = listeners.indexOf(handler);
      if (index >= 0) listeners.splice(index, 1);
    },
  };
  if (!window.chrome) window.chrome = {};
  try {
    Object.defineProperty(window.chrome, 'webview', { value: webview, configurable: true });
  } catch (error) {
    window.chrome = { webview: webview };
  }

  window.addEventListener('message', function (event) {
    if (event.source !== window.parent) return;
    var data = event.data;
    if (data && typeof data === 'object' && data.type === 'theme' && typeof data.css === 'string') {
      var style = document.getElementById('fk-theme');
      if (style) style.textContent = data.css;
    }
    if (data && typeof data === 'object' && data.type === 'hostKey' && typeof data.key === 'string') {
      (document.activeElement || document.body).dispatchEvent(new KeyboardEvent('keydown', { key: data.key, code: data.key, bubbles: true, cancelable: true }));
      return;
    }
    listeners.slice().forEach(function (handler) {
      handler({ data: data });
    });
    if (data && typeof data === 'object') window.dispatchEvent(new MessageEvent('flowkey-message', { data: data }));
  });
  document.addEventListener('mouseup', function (event) {
    var target = event.target;
    if (target && target.closest && target.closest('input,textarea,select,[contenteditable]:not([contenteditable="false"])')) return;
    webview.postMessage({ type: 'focusHost' });
  }, true);
  window.addEventListener('error', function (event) {
    webview.postMessage({ type: 'log', level: 'error', message: 'webview page error: ' + event.message });
  });
  window.addEventListener('unhandledrejection', function (event) {
    webview.postMessage({ type: 'log', level: 'error', message: 'webview unhandled rejection: ' + event.reason });
  });

  var pending = new Map();
  var sequence = 1;
  webview.addEventListener('message', function (event) {
    var data = event.data;
    if (!data || data.type !== 'result' || !pending.has(data.bridgeId)) return;
    var settle = pending.get(data.bridgeId);
    pending.delete(data.bridgeId);
    settle(data);
  });

  function call(method, params, options) {
    var bridgeId = 'h' + sequence++;
    var signal = options && options.signal;
    return new Promise(function (resolve, reject) {
      var onAbort = function () {
        pending.delete(bridgeId);
        reject({ code: 'aborted', message: 'native method ' + method + ' aborted' });
        webview.postMessage({ type: 'webAbort', bridgeId: bridgeId, extensionId: extensionId });
      };
      if (signal) signal.addEventListener('abort', onAbort, { once: true });
      pending.set(bridgeId, function (message) {
        if (signal) signal.removeEventListener('abort', onAbort);
        if (message.ok) resolve(message.result);
        else reject(message.error || { code: 'webFailed', message: 'web call failed' });
      });
      webview.postMessage({ type: 'webCall', bridgeId: bridgeId, extensionId: extensionId, method: method, params: params, timeoutMs: options && options.timeoutMs });
    });
  }

  function field(name, fallback) {
    return function (result) {
      if (result && result.ok === false) throw new Error('native call failed');
      return result && result[name] !== undefined && result[name] !== null ? result[name] : fallback;
    };
  }

  var capabilities = {
    http: {
      fetch: function (url, options) {
        options = options || {};
        return call('http.fetch', { url: url, method: options.method, headers: options.headers, body: options.body, auth: options.auth, discardBody: options.discardBody, timeoutMs: options.timeoutMs }, { signal: options.signal });
      },
      fetchJson: function (url, options) {
        return capabilities.http.fetch(url, options).then(function (r) {
          return JSON.parse(r.bodyText || '{}');
        });
      },
      upload: function (url, options) {
        options = options || {};
        return call('http.upload', { url: url, bytes: options.bytes, method: options.method, headers: options.headers, timeoutMs: options.timeoutMs }, { signal: options.signal });
      },
    },
    clipboard: {
      read: function () { return call('clipboard.read').then(field('text', null)); },
      write: function (text) { return call('clipboard.write', { text: text }); },
      paste: function (text) { return call('clipboard.paste', { text: text }); },
      history: function (o) { return call('clipboard.history', { query: o && o.query, limit: o && o.limit }).then(field('items', [])); },
      clearHistory: function () { return call('clipboard.clearHistory', {}); },
      deleteEntry: function (id) { return call('clipboard.deleteEntry', { id: id }); },
      copyEntry: function (id) { return call('clipboard.copyEntry', { id: id }); },
      pasteEntry: function (id) { return call('clipboard.pasteEntry', { id: id }); },
      editEntry: function (id) { return call('clipboard.editEntry', { id: id }); },
      writeContent: function (content) { return call('clipboard.write', content); },
      readContent: function () { return call('clipboard.read'); },
      clear: function () { return call('clipboard.clear', {}); },
    },
    storage: {
      get: function (key) { return call('storage.get', { key: key }).then(field('value', null)); },
      set: function (key, value) { return call('storage.set', { key: key, value: value }); },
      delete: function (key) { return call('storage.delete', { key: key }); },
      keys: function () { return call('storage.keys').then(field('keys', [])); },
      allItems: function () {
        return capabilities.storage.keys().then(function (keys) {
          return Promise.all(keys.map(function (key) {
            return capabilities.storage.get(key).then(function (value) { return [key, value]; });
          })).then(function (entries) {
            var out = {};
            entries.forEach(function (e) { if (e[1] !== null) out[e[0]] = e[1]; });
            return out;
          });
        });
      },
    },
    cache: {
      get: function (key) { return call('cache.get', { key: key }).then(field('value', null)); },
      set: function (key, value, o) { return call('cache.set', { key: key, value: value, ttlSeconds: o && o.ttlSeconds }); },
      delete: function (key) { return call('cache.delete', { key: key }); },
      clear: function () { return call('cache.clear', {}); },
    },
    secrets: {
      get: function (key) { return call('secrets.get', { key: key }).then(field('value', null)); },
      set: function (key, value) { return call('secrets.set', { key: key, value: value }); },
      delete: function (key) { return call('secrets.delete', { key: key }); },
    },
    shell: {
      openUrl: function (url) { return call('shell.openUrl', { url: url }); },
      openPath: function (path) { return call('shell.openPath', { path: path }); },
      revealPath: function (path) { return call('shell.revealPath', { path: path }); },
    },
    fs: {
      readText: function (path) { return call('fs.readText', { path: path }).then(field('content', '')); },
      writeText: function (path, content, o) { return call('fs.writeText', { path: path, content: content, append: !!(o && o.append) }); },
      delete: function (path) { return call('fs.delete', { path: path }); },
      glob: function (pattern, o) { return call('fs.glob', { pattern: pattern, limit: o && o.limit }).then(field('entries', [])); },
      stat: function (path) { return call('fs.stat', { path: path }); },
      mkdir: function (path) { return call('fs.mkdir', { path: path }); },
      exists: function (path) { return call('fs.exists', { path: path }).then(function (r) { return r.exists === true; }); },
      copy: function (from, to) { return call('fs.copy', { from: from, to: to }); },
      move: function (from, to) { return call('fs.move', { from: from, to: to }); },
      trash: function (path) { return call('fs.trash', { path: path }); },
    },
    apps: {
      list: function (query) { return call('apps.list', { query: query }).then(field('apps', [])); },
      launch: function (id) { return call('apps.launch', { id: id }); },
      frontmost: function () { return call('apps.frontmost').then(field('app', null)); },
      defaultFor: function (path) { return call('apps.default', { path: path }).then(field('path', null)); },
    },
    media: {
      current: function () { return call('media.current'); },
      control: function (command) { return call('media.control', { command: command }); },
    },
    image: {
      fetch: function (url) { return call('image.fetch', { url: url }).then(field('uri', '')); },
    },
    system: {
      selectedText: function (o) { return call('system.selectedText', { allowFallback: !!(o && o.allowFallback) }).then(field('text', null)); },
    },
    oauth: {
      authorize: function (provider, o) { return call('oauth.authorize', { provider: provider, clientId: o && o.clientId }, { signal: o && o.signal, timeoutMs: o && o.timeoutMs }); },
      status: function (provider, o) { return call('oauth.status', { provider: provider }, o); },
      disconnect: function (provider) { return call('oauth.disconnect', { provider: provider }); },
    },
    hud: { show: function (o) { return call('hud.show', o); } },
    toast: { show: function (o) { return call('toast.show', o); } },
    alert: { confirm: function (o) { return call('alert.confirm', o).then(function (r) { return r.confirmed === true; }); } },
  };

  window.flowkeyPost = function (message) {
    webview.postMessage(message);
  };
  window.flowkey = {
    extensionId: extensionId,
    post: function (message) {
      webview.postMessage(message);
    },
    onMessage: function (handler) {
      webview.addEventListener('message', function (event) {
        var data = event.data;
        if (data && typeof data === 'object' && data.type !== 'theme') handler(data);
      });
    },
    capabilities: capabilities,
  };
})();

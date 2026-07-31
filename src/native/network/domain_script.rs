// SPDX-License-Identifier: MIT OR Apache-2.0
//! Injected JS for domain allow-list (Workers/fetch/XHR/WS).

pub(crate) fn domain_filter_script(allowed_domains: &[String]) -> String {
    let domains_json = serde_json::to_string(allowed_domains).unwrap_or("[]".to_string());
    format!(
        r#"(() => {{
            const _allowed = {domains_json};
            function _installDomainFilter(_allowed, _baseOverride) {{
            const _global = globalThis;
            function _securityError(message) {{
                if (typeof DOMException === 'function') {{
                    return new DOMException(message, 'SecurityError');
                }}
                const error = new Error(message);
                error.name = 'SecurityError';
                return error;
            }}
            function _isDomainAllowed(hostname) {{
                hostname = hostname.toLowerCase();
                for (const p of _allowed) {{
                    if (p.startsWith('*.')) {{
                        const suffix = p.slice(2);
                        if (hostname === suffix || hostname.endsWith('.' + suffix)) return true;
                    }} else if (hostname === p) return true;
                }}
                return false;
            }}
            const _baseHref = _baseOverride || (_global.location && _global.location.href ? _global.location.href : 'about:blank');
            function _checkedUrl(url, apiName) {{
                const u = new URL(url, _baseHref);
                if (['http:', 'https:', 'ws:', 'wss:'].includes(u.protocol) && !_isDomainAllowed(u.hostname)) {{
                    throw _securityError(apiName + ' blocked: ' + u.hostname);
                }}
                return u.href;
            }}
            function _assertAllowedUrl(url, apiName) {{
                _checkedUrl(url, apiName);
            }}
            function _checkedWebSocketUrl(url, apiName) {{
                const u = new URL(url, _baseHref);
                if (u.protocol === 'http:') u.protocol = 'ws:';
                if (u.protocol === 'https:') u.protocol = 'wss:';
                if (['ws:', 'wss:'].includes(u.protocol) && !_isDomainAllowed(u.hostname)) {{
                    throw _securityError(apiName + ' blocked: ' + u.hostname);
                }}
                return u.href;
            }}
            function _requestUrl(input) {{
                if (typeof input === 'string') return input;
                if (typeof URL === 'function' && input instanceof URL) return input.href;
                if (input && typeof input.url === 'string') return input.url;
                return String(input);
            }}
            const _workerUrlCache = typeof Map === 'function' ? new Map() : null;
            function _checkedWorkerScriptUrl(scriptURL, apiName) {{
                let absolute;
                try {{
                    const u = new URL(scriptURL, _baseHref);
                    absolute = u.href;
                    if (u.protocol === 'blob:') {{
                        try {{
                            const inner = new URL(u.pathname);
                            if (inner.hostname && !_isDomainAllowed(inner.hostname)) {{
                                throw _securityError(apiName + ' blocked: ' + inner.hostname);
                            }}
                        }} catch(e) {{ if (e && e.name === 'SecurityError') throw e; }}
                    }} else if (u.hostname && !_isDomainAllowed(u.hostname)) {{
                        throw _securityError(apiName + ' blocked: ' + u.hostname);
                    }}
                }} catch(e) {{
                    if (e && e.name === 'SecurityError') throw e;
                    throw e;
                }}
                return absolute;
            }}
            function _workerScriptUrl(scriptURL, options, apiName) {{
                if (!_global.Blob || !_global.URL || typeof _global.URL.createObjectURL !== 'function') {{
                    throw _securityError(apiName + ' blocked: worker bootstrap APIs are unavailable');
                }}
                const absolute = _checkedWorkerScriptUrl(scriptURL, apiName);
                const isModule = options && typeof options === 'object' && options.type === 'module';
                const cacheKey = apiName + '|' + (isModule ? 'module' : 'classic') + '|' + absolute;
                if (_workerUrlCache && _workerUrlCache.has(cacheKey)) return _workerUrlCache.get(cacheKey);
                const installSource = '(' + _installDomainFilter.toString() + ')(' + JSON.stringify(_allowed) + ', ' + JSON.stringify(absolute) + ');\n';
                const source = installSource + (isModule
                    ? 'await import(' + JSON.stringify(absolute) + ');\n'
                    : 'importScripts(' + JSON.stringify(absolute) + ');\n');
                const wrapped = _global.URL.createObjectURL(new Blob([source], {{ type: 'application/javascript' }}));
                if (_workerUrlCache) _workerUrlCache.set(cacheKey, wrapped);
                return wrapped;
            }}
            function _constructWorker(OrigCtor, scriptURL, options, apiName) {{
                const checkedUrl = _checkedWorkerScriptUrl(scriptURL, apiName);
                try {{
                    const bootstrapUrl = _workerScriptUrl(checkedUrl, options, apiName);
                    const worker = new OrigCtor(bootstrapUrl, options);
                    return worker;
                }} catch (error) {{
                    // Fail closed if the guarded bootstrap cannot be created.
                    throw error;
                }}
            }}
            const OrigWorker = _global.Worker;
            if (typeof OrigWorker === 'function') {{
                _global.Worker = function(scriptURL, options) {{
                    return _constructWorker(OrigWorker, scriptURL, options, 'Worker');
                }};
                _global.Worker.prototype = OrigWorker.prototype;
            }}
            const OrigSharedWorker = _global.SharedWorker;
            if (typeof OrigSharedWorker === 'function') {{
                _global.SharedWorker = function(scriptURL, options) {{
                    return _constructWorker(OrigSharedWorker, scriptURL, options, 'SharedWorker');
                }};
                _global.SharedWorker.prototype = OrigSharedWorker.prototype;
            }}
            const OrigImportScripts = _global.importScripts;
            if (typeof OrigImportScripts === 'function') {{
                _global.importScripts = function() {{
                    const urls = Array.prototype.slice.call(arguments).map((url) => {{
                        try {{
                            return _checkedUrl(url, 'importScripts');
                        }} catch(e) {{
                            if (e && e.name === 'SecurityError') throw e;
                            return url;
                        }}
                    }});
                    return OrigImportScripts.apply(this, urls);
                }};
            }}
            const OrigFetch = _global.fetch;
            if (typeof OrigFetch === 'function') {{
                _global.fetch = function(input, init) {{
                    try {{
                        if (typeof input === 'string') {{
                            return OrigFetch.call(this, _checkedUrl(input, 'Fetch'), init);
                        }}
                        _assertAllowedUrl(_requestUrl(input), 'Fetch');
                    }} catch(e) {{
                        if (e && e.name === 'SecurityError') return Promise.reject(e);
                    }}
                    return OrigFetch.apply(this, arguments);
                }};
            }}
            const OrigXHR = _global.XMLHttpRequest;
            if (typeof OrigXHR === 'function' && OrigXHR.prototype && OrigXHR.prototype.open) {{
                const origOpen = OrigXHR.prototype.open;
                OrigXHR.prototype.open = function(method, url) {{
                    let checkedUrl = url;
                    try {{
                        checkedUrl = _checkedUrl(url, 'XMLHttpRequest');
                    }} catch(e) {{
                        if (e && e.name === 'SecurityError') throw e;
                    }}
                    const args = Array.prototype.slice.call(arguments);
                    args[1] = checkedUrl;
                    return origOpen.apply(this, args);
                }};
            }}
            const OrigWS = _global.WebSocket;
            if (typeof OrigWS === 'function') {{
                _global.WebSocket = function(url, protocols) {{
                    let checkedUrl = url;
                    try {{
                        checkedUrl = _checkedWebSocketUrl(url, 'WebSocket');
                    }} catch(e) {{ if (e && e.name === 'SecurityError') throw e; }}
                    return new OrigWS(checkedUrl, protocols);
                }};
                _global.WebSocket.prototype = OrigWS.prototype;
            }}
            const OrigES = _global.EventSource;
            if (OrigES) {{
                _global.EventSource = function(url, opts) {{
                    let checkedUrl = url;
                    try {{
                        checkedUrl = _checkedUrl(url, 'EventSource');
                    }} catch(e) {{ if (e && e.name === 'SecurityError') throw e; }}
                    return new OrigES(checkedUrl, opts);
                }};
                _global.EventSource.prototype = OrigES.prototype;
            }}
            const origBeacon = _global.navigator && _global.navigator.sendBeacon;
            if (origBeacon) {{
                _global.navigator.sendBeacon = function(url, data) {{
                    let checkedUrl = url;
                    try {{
                        checkedUrl = _checkedUrl(url, 'Beacon');
                    }} catch(e) {{ return false; }}
                    return origBeacon.call(_global.navigator, checkedUrl, data);
                }};
            }}
            function _blockPeerConnection(name) {{
                if (typeof _global[name] !== 'function') return;
                const BlockedPeerConnection = function() {{
                    throw _securityError('RTCPeerConnection blocked while domain filtering is active');
                }};
                Object.defineProperty(BlockedPeerConnection, 'prototype', {{
                    value: Object.freeze(Object.create(null)),
                    writable: false
                }});
                try {{
                    Object.defineProperty(_global, name, {{
                        value: BlockedPeerConnection,
                        writable: false,
                        configurable: false
                    }});
                }} catch (_) {{
                    _global[name] = BlockedPeerConnection;
                }}
            }}
            _blockPeerConnection('RTCPeerConnection');
            _blockPeerConnection('webkitRTCPeerConnection');
            }}
            _installDomainFilter(_allowed);
        }})()"#,
    )
}

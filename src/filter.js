(function () {
    const signatures = __SIGNATURES__;
    const allowSW = __ALLOW_SW__;

    /**
     * Логирование в терминал Rust
     */
    const log = (msg) => {
        const payload = 'filter:' + msg;
        if (window.chrome && window.chrome.webview) {
            window.chrome.webview.postMessage(payload);
        } else if (window.ipc) {
            window.ipc.postMessage(payload);
        }
    };

    // Звуковые ресурсы из Rust
    const SOUNDS_B64 = {
        'msg': "__NOTIF_B64__",
        'call_incoming': "__RING_B64__",
        'call_reconnect': "__RECON_B64__",
    };

    /**
     * Блокировка инициализации AppTracer
     */
    const neuterSDKs = () => {
        const dummy = {
            instance: {}, modules: {}, session: {},
            log: () => { }, error: () => { },
            performance: { time: () => { }, timeEnd: () => { }, addSample: () => { } },
            plugins: { addPlugin: () => () => { } },
            setUserId: () => { }, setErrorKey: () => { },
        };
        ['TracerSDK2', 'tracerMain', '_tracer'].forEach(key => {
            try {
                Object.defineProperty(window, key, { value: dummy, writable: false, configurable: false });
            } catch (e) { window[key] = dummy; }
        });
    };

    /**
     * Декодирование Base64 в Blob
     */
    function b64ToBlob(b64Data, contentType = 'audio/mpeg') {
        if (!b64Data || b64Data.length < 50) return null;
        try {
            const byteCharacters = atob(b64Data);
            const byteArrays = [];
            for (let offset = 0; offset < byteCharacters.length; offset += 512) {
                const slice = byteCharacters.slice(offset, offset + 512);
                const byteNumbers = new Array(slice.length);
                for (let i = 0; i < slice.length; i++) {
                    byteNumbers[i] = slice.charCodeAt(i);
                }
                const byteArray = new Uint8Array(byteNumbers);
                byteArrays.push(byteArray);
            }
            return new Blob(byteArrays, { type: contentType });
        } catch (e) {
            log('b64_error:' + e.message);
            return null;
        }
    }

    const soundBlobs = {};
    Object.keys(SOUNDS_B64).forEach(k => {
        soundBlobs[k] = b64ToBlob(SOUNDS_B64[k]);
    });

    /**
     * Определение замены аудио
     */
    function getReplacementBlob(url) {
        if (!url || typeof url !== 'string') return null;
        const u = url.toLowerCase();
        
        if (u.includes('msg') || u.includes('notification')) return soundBlobs['msg'];
        if (u.includes('call_incoming') || u.includes('ringtone')) return soundBlobs['call_incoming'];
        if (u.includes('reconnect')) return soundBlobs['call_reconnect'];
        
        return null;
    }

    // --- Перехватчики ---
    neuterSDKs();

    const OriginalAudio = window.Audio;
    window.Audio = function (src) {
        const blob = getReplacementBlob(src);
        if (blob) {
            src = URL.createObjectURL(blob);
            log('replacing_audio:Audio_const');
        }
        return new OriginalAudio(src);
    };
    window.Audio.prototype = OriginalAudio.prototype;

    const _fetch = window.fetch;
    window.fetch = function (input, init) {
        const url = (typeof input === 'string' ? input : (input && input.url) || '');
        const blob = getReplacementBlob(url);
        if (blob) {
            log('replacing_audio:fetch:' + url);
            return Promise.resolve(new Response(blob, {
                status: 200, statusText: 'OK',
                headers: { 'Content-Type': 'audio/mpeg' }
            }));
        }
        if (signatures.some(sig => url.toLowerCase().includes(sig))) {
            return Promise.reject(new Error('Blocked'));
        }
        return _fetch.apply(this, arguments);
    };

    const _xhrOpen = XMLHttpRequest.prototype.open;
    XMLHttpRequest.prototype.open = function (method, url) {
        const blob = getReplacementBlob(url);
        this._replacement = blob ? URL.createObjectURL(blob) : null;
        this._url = this._replacement || url;
        const lowUrl = (typeof url === 'string' ? url : '').toLowerCase();
        this._blocked = signatures.some(sig => lowUrl.includes(sig));
        return _xhrOpen.call(this, method, this._url);
    };

    const _xhrSend = XMLHttpRequest.prototype.send;
    XMLHttpRequest.prototype.send = function () {
        if (this._blocked) { this.abort(); return; }
        if (this._replacement) log('replacing_audio:xhr:' + this._url);
        return _xhrSend.apply(this, arguments);
    };

    // --- Визуальные исправления (Анимация) ---
    const visualFix = document.createElement('style');
    visualFix.innerHTML = `
        div[class^="glow"], 
        div[class*=" glow"],
        div[class^="animation-layer-wrapper"],
        div[class*=" animation-layer-wrapper"] {
            display: none !important;
            visibility: hidden !important;
            opacity: 0 !important;
            animation: none !important;
            pointer-events: none !important;
        }
        div[class^="preview"] *, 
        div[class*=" preview"] * {
            animation: none !important;
            transition: none !important;
        }
    `;
    const injectStyles = () => {
        if (document.head) document.head.appendChild(visualFix);
        else setTimeout(injectStyles, 10);
    };
    injectStyles();

    // Остальные фильтры
    if (navigator.geolocation) {
        navigator.geolocation.getCurrentPosition = (_s, e) => { if (e) e({ code: 1, message: "Denied" }); };
    }
    if (!allowSW && navigator.serviceWorker) {
        navigator.serviceWorker.register = () => Promise.reject();
    }
    const _beacon = navigator.sendBeacon;
    navigator.sendBeacon = function (url) {
        if (signatures.some(sig => url.toLowerCase().includes(sig))) return false;
        return _beacon.apply(this, arguments);
    };

    // Сигнал готовности
    const sendReady = () => {
        log('ready');
    };
    if (document.readyState === 'complete' || document.readyState === 'interactive') {
        setTimeout(sendReady, 100);
    } else {
        window.addEventListener('DOMContentLoaded', () => setTimeout(sendReady, 100));
    }

    log('filter_initialized_v2');
})();
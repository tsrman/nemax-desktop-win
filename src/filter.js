(function () {
    const signatures = __SIGNATURES__;
    const allowSW    = __ALLOW_SW__;

    const ipc = window.ipc;
    const log = (msg) => { if (ipc) ipc.postMessage('filter:' + msg); };

    // Звуковые данные из Rust
    const SOUNDS_B64 = {
        'msg': "__NOTIF_B64__",          // сообщение
        'call_incoming': "__RING_B64__", // звонок
        'call_reconnect': "__RECON_B64__",   // восстановление звонка
    };

    /**
     * Помощник для преобразования Base64 в Blob (надежнее для AudioContext)
     */
    function b64ToBlob(b64Data, contentType = 'audio/mpeg') {
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

    /**
     * Ищет замену и возвращает НОВЫЙ Blob каждый раз
     */
    function getReplacementBlob(url) {
        if (!url || typeof url !== 'string') return null;
        const u = url.toLowerCase();
        for (const [key, b64] of Object.entries(SOUNDS_B64)) {
            if (b64 && b64.length > 10 && u.includes(key)) {
                log('replacing_audio:' + key);
                return b64ToBlob(b64);
            }
        }
        return null;
    }

    // 1. Перехват конструктора Audio (для простых звуков)
    const OriginalAudio = window.Audio;
    window.Audio = function (src) {
        const blob = getReplacementBlob(src);
        if (blob) {
            src = URL.createObjectURL(blob);
        }
        return new OriginalAudio(src);
    };

    // 2. Перехват fetch (основной метод для современных плееров)
    const _fetch = window.fetch;
    window.fetch = function (input, init) {
        let url = (typeof input === 'string' ? input : (input && input.url) || '');
        const blob = getReplacementBlob(url);
        
        if (blob) {
            // Создаем абсолютно новый Response из Blob
            // Это решает проблему "detached ArrayBuffer"
            return Promise.resolve(new Response(blob, {
                status: 200,
                statusText: 'OK',
                headers: { 'Content-Type': 'audio/mpeg' }
            }));
        }

        const lowerUrl = url.toLowerCase();
        if (signatures.some(sig => lowerUrl.includes(sig))) {
            log('fetch_blocked:' + url);
            return Promise.reject(new Error('Request blocked'));
        }
        return _fetch.apply(this, arguments);
    };

    // 3. Перехват XMLHttpRequest
    const _xhrOpen = XMLHttpRequest.prototype.open;
    XMLHttpRequest.prototype.open = function (method, url) {
        const blob = getReplacementBlob(url);
        if (blob) {
            url = URL.createObjectURL(blob);
        }

        let u = (typeof url === 'string' ? url : '').toLowerCase();
        this._blocked = signatures.some(sig => u.includes(sig));
        return _xhrOpen.apply(this, arguments);
    };

    // --- Динамическое скрытие эффектов звонка без привязки к хешам ---
    const visualFix = document.createElement('style');
    visualFix.innerHTML = `
        /* Скрываем элементы, классы которых начинаются на glow или animation-layer-wrapper */
        /* внутри контейнера звонка */
        
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

        /* Полная остановка любых анимаций в блоке превью звонка */
        div[class^="preview"] *, 
        div[class*=" preview"] * {
            animation: none !important;
            transition: none !important;
        }
    `;
    
    // Функция для безопасной вставки стилей (ждем появления head)
    const injectStyles = () => {
        if (document.head) {
            document.head.appendChild(visualFix);
        } else {
            setTimeout(injectStyles, 10);
        }
    };
    injectStyles();
    
    log('dynamic_visual_fix_applied');

    // --- Остальные фильтры (Геолокация, SW, Beacon) ---
    if (navigator.geolocation) {
        navigator.geolocation.getCurrentPosition = (_s, error) => {
            if (error) error({ code: 1, message: 'Access denied' });
        };
        navigator.geolocation.watchPosition = () => null;
    }

    if (!allowSW && navigator.serviceWorker) {
        navigator.serviceWorker.register = () => Promise.reject(new Error('SW disabled'));
    }

    const _xhrSend = XMLHttpRequest.prototype.send;
    XMLHttpRequest.prototype.send = function (body) {
        if (this._blocked) { this.abort(); return; }
        return _xhrSend.apply(this, arguments);
    };

    const _beacon = navigator.sendBeacon;
    navigator.sendBeacon = function (url, data) {
        if (signatures.some(sig => url.toLowerCase().includes(sig))) return false;
        return _beacon.apply(this, arguments);
    };

// Функция отправки сигнала готовности
    const sendReadySignal = () => {
        if (window.ipc) {
            window.ipc.postMessage('filter:ready');
            log('ready_signal_sent');
        }
    };

    // Если страница уже загружена (interactive или complete), шлем сразу.
    // Если нет — ждем события.
    if (document.readyState === 'complete' || document.readyState === 'interactive') {
        setTimeout(sendReadySignal, 100);
    } else {
        window.addEventListener('DOMContentLoaded', () => {
            setTimeout(sendReadySignal, 100);
        });
    }

    log('filter_initialized_v2');
})();
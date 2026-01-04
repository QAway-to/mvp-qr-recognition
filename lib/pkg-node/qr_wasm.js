
let imports = {};
imports['__wbindgen_placeholder__'] = module.exports;

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
function decodeText(ptr, len) {
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    }
}

let WASM_VECTOR_LEN = 0;

const WasmQRScannerFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => { }, unregister: () => { } }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmqrscanner_free(ptr >>> 0, 1));

/**
 * JavaScript-доступный сканер QR-кодов
 */
class WasmQRScanner {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(WasmQRScanner.prototype);
        obj.__wbg_ptr = ptr;
        WasmQRScannerFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmQRScannerFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmqrscanner_free(ptr, 0);
    }
    /**
     * Сканирование изображения из байтов (PNG, JPEG)
     *
     * @param image_data - Uint8Array с данными изображения
     * @returns Object с результатами сканирования
     * @param {Uint8Array} image_data
     * @returns {any}
     */
    scanImage(image_data) {
        const ptr0 = passArray8ToWasm0(image_data, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmqrscanner_scanImage(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Создание сканера с пользовательскими настройками
     * @param {boolean} adaptive_threshold
     * @param {number} block_size
     * @param {boolean} denoise
     * @param {number} denoise_strength
     * @param {boolean} enhance_contrast
     * @returns {WasmQRScanner}
     */
    static withConfig(adaptive_threshold, block_size, denoise, denoise_strength, enhance_contrast) {
        const ret = wasm.wasmqrscanner_withConfig(adaptive_threshold, block_size, denoise, denoise_strength, enhance_contrast);
        return WasmQRScanner.__wrap(ret);
    }
    /**
     * Сканирование ImageData из Canvas
     *
     * @param data - Uint8ClampedArray из canvas.getImageData()
     * @param width - Ширина изображения
     * @param height - Высота изображения
     * @returns Object с результатами сканирования
     * @param {Uint8Array} data
     * @param {number} width
     * @param {number} height
     * @returns {any}
     */
    scanImageData(data, width, height) {
        const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmqrscanner_scanImageData(this.__wbg_ptr, ptr0, len0, width, height);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Поиск платёжного QR-кода
     *
     * @param image_data - Uint8Array с данными изображения
     * @returns PaymentInfo или null
     * @param {Uint8Array} image_data
     * @returns {any}
     */
    scanForPayment(image_data) {
        const ptr0 = passArray8ToWasm0(image_data, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmqrscanner_scanForPayment(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Создание нового сканера с настройками по умолчанию
     */
    constructor() {
        const ret = wasm.wasmqrscanner_new();
        this.__wbg_ptr = ret >>> 0;
        WasmQRScannerFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
}
if (Symbol.dispose) WasmQRScanner.prototype[Symbol.dispose] = WasmQRScanner.prototype.free;
exports.WasmQRScanner = WasmQRScanner;

/**
 * Удобная функция для быстрого сканирования
 * @param {Uint8Array} image_data
 * @returns {any}
 */
function quickScan(image_data) {
    const ptr0 = passArray8ToWasm0(image_data, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.quickScan(ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}
exports.quickScan = quickScan;

/**
 * Инициализация panic hook для отладки
 */
function start() {
    wasm.start();
}
exports.start = start;

/**
 * Информация о версии
 * @returns {string}
 */
function version() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.version();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}
exports.version = version;

exports.__wbg_Error_52673b7de5a0ca89 = function (arg0, arg1) {
    const ret = Error(getStringFromWasm0(arg0, arg1));
    return ret;
};

exports.__wbg_String_8f0eb39a4a4c2f66 = function (arg0, arg1) {
    const ret = String(arg1);
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
};

exports.__wbg___wbindgen_is_string_704ef9c8fc131030 = function (arg0) {
    const ret = typeof (arg0) === 'string';
    return ret;
};

exports.__wbg___wbindgen_throw_dd24417ed36fc46e = function (arg0, arg1) {
    throw new Error(getStringFromWasm0(arg0, arg1));
};

exports.__wbg_debug_9d0c87ddda3dc485 = function (arg0) {
    console.debug(arg0);
};

exports.__wbg_error_7534b8e9a36f1ab4 = function (arg0, arg1) {
    let deferred0_0;
    let deferred0_1;
    try {
        deferred0_0 = arg0;
        deferred0_1 = arg1;
        console.error(getStringFromWasm0(arg0, arg1));
    } finally {
        wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
    }
};

exports.__wbg_error_7bc7d576a6aaf855 = function (arg0) {
    console.error(arg0);
};

exports.__wbg_getTime_ad1e9878a735af08 = function (arg0) {
    const ret = arg0.getTime();
    return ret;
};

exports.__wbg_info_ce6bcc489c22f6f0 = function (arg0) {
    console.info(arg0);
};

exports.__wbg_log_1d990106d99dacb7 = function (arg0) {
    console.log(arg0);
};

exports.__wbg_new_0_23cedd11d9b40c9d = function () {
    const ret = new Date();
    return ret;
};

exports.__wbg_new_1ba21ce319a06297 = function () {
    const ret = new Object();
    return ret;
};

exports.__wbg_new_25f239778d6112b9 = function () {
    const ret = new Array();
    return ret;
};

exports.__wbg_new_8a6f238a6ece86ea = function () {
    const ret = new Error();
    return ret;
};

exports.__wbg_new_b546ae120718850e = function () {
    const ret = new Map();
    return ret;
};

exports.__wbg_set_3f1d0b984ed272ed = function (arg0, arg1, arg2) {
    arg0[arg1] = arg2;
};

exports.__wbg_set_7df433eea03a5c14 = function (arg0, arg1, arg2) {
    arg0[arg1 >>> 0] = arg2;
};

exports.__wbg_set_efaaf145b9377369 = function (arg0, arg1, arg2) {
    const ret = arg0.set(arg1, arg2);
    return ret;
};

exports.__wbg_stack_0ed75d68575b0f3c = function (arg0, arg1) {
    const ret = arg1.stack;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
};

exports.__wbg_warn_6e567d0d926ff881 = function (arg0) {
    console.warn(arg0);
};

exports.__wbindgen_cast_2241b6af4c4b2941 = function (arg0, arg1) {
    // Cast intrinsic for `Ref(String) -> Externref`.
    const ret = getStringFromWasm0(arg0, arg1);
    return ret;
};

exports.__wbindgen_cast_4625c577ab2ec9ee = function (arg0) {
    // Cast intrinsic for `U64 -> Externref`.
    const ret = BigInt.asUintN(64, arg0);
    return ret;
};

exports.__wbindgen_cast_d6cd19b81560fd6e = function (arg0) {
    // Cast intrinsic for `F64 -> Externref`.
    const ret = arg0;
    return ret;
};

exports.__wbindgen_init_externref_table = function () {
    const table = wasm.__wbindgen_externrefs;
    const offset = table.grow(4);
    table.set(0, undefined);
    table.set(offset + 0, undefined);
    table.set(offset + 1, null);
    table.set(offset + 2, true);
    table.set(offset + 3, false);
};

const wasmPath = `${__dirname}/qr_wasm_bg.wasm`;
const wasmBytes = require('fs').readFileSync(wasmPath);
const wasmModule = new WebAssembly.Module(wasmBytes);
const wasm = exports.__wasm = new WebAssembly.Instance(wasmModule, imports).exports;

wasm.__wbindgen_start();

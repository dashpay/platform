"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    Object.defineProperty(o, k2, { enumerable: true, get: function() { return m[k]; } });
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const wasm_dpp_1 = __importDefault(require("../wasm/wasm_dpp"));
const dpp_module = __importStar(require("./dpp"));
// @ts-ignore
const wasm_dpp_bg_js_1 = __importDefault(require("../wasm/wasm_dpp_bg.js"));
let isInitialized = false;
let loadingPromise = null;
function loadDpp() {
    return __awaiter(this, void 0, void 0, function* () {
        if (isInitialized) {
            return dpp_module;
        }
        if (!loadingPromise) {
            loadingPromise = loadDppModule();
        }
        yield loadingPromise;
        isInitialized = true;
        loadingPromise = null;
        return dpp_module;
    });
}
exports.default = loadDpp;
;
const loadDppModule = () => __awaiter(void 0, void 0, void 0, function* () {
    // @ts-ignore
    let bytes = Buffer.from(wasm_dpp_bg_js_1.default, 'base64');
    if (typeof window !== 'undefined') {
        let blob = new Blob([bytes], { type: "application/wasm" });
        let wasmUrl = URL.createObjectURL(blob);
        yield wasm_dpp_1.default(wasmUrl);
    }
    else {
        dpp_module.initSync(bytes);
    }
});

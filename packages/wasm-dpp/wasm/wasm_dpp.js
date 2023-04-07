import { default as default1 } from '../lib/identifier/Identifier.js';
import { set } from 'lodash';

let wasm;

const cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });

cachedTextDecoder.decode();

let cachedUint8Memory0 = null;

function getUint8Memory0() {
    if (cachedUint8Memory0 === null || cachedUint8Memory0.byteLength === 0) {
        cachedUint8Memory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8Memory0;
}

function getStringFromWasm0(ptr, len) {
    return cachedTextDecoder.decode(getUint8Memory0().subarray(ptr, ptr + len));
}

const heap = new Array(128).fill(undefined);

heap.push(undefined, null, true, false);

let heap_next = heap.length;

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

function getObject(idx) { return heap[idx]; }

function isLikeNone(x) {
    return x === undefined || x === null;
}

let cachedFloat64Memory0 = null;

function getFloat64Memory0() {
    if (cachedFloat64Memory0 === null || cachedFloat64Memory0.byteLength === 0) {
        cachedFloat64Memory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachedFloat64Memory0;
}

let cachedInt32Memory0 = null;

function getInt32Memory0() {
    if (cachedInt32Memory0 === null || cachedInt32Memory0.byteLength === 0) {
        cachedInt32Memory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachedInt32Memory0;
}

let WASM_VECTOR_LEN = 0;

const cachedTextEncoder = new TextEncoder('utf-8');

const encodeString = (typeof cachedTextEncoder.encodeInto === 'function'
    ? function (arg, view) {
    return cachedTextEncoder.encodeInto(arg, view);
}
    : function (arg, view) {
    const buf = cachedTextEncoder.encode(arg);
    view.set(buf);
    return {
        read: arg.length,
        written: buf.length
    };
});

function passStringToWasm0(arg, malloc, realloc) {

    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length);
        getUint8Memory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len);

    const mem = getUint8Memory0();

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
        ptr = realloc(ptr, len, len = offset + arg.length * 3);
        const view = getUint8Memory0().subarray(ptr + offset, ptr + len);
        const ret = encodeString(arg, view);

        offset += ret.written;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function dropObject(idx) {
    if (idx < 132) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

let cachedBigInt64Memory0 = null;

function getBigInt64Memory0() {
    if (cachedBigInt64Memory0 === null || cachedBigInt64Memory0.byteLength === 0) {
        cachedBigInt64Memory0 = new BigInt64Array(wasm.memory.buffer);
    }
    return cachedBigInt64Memory0;
}

function makeMutClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {
        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            if (--state.cnt === 0) {
                wasm.__wbindgen_export_2.get(state.dtor)(a, state.b);

            } else {
                state.a = a;
            }
        }
    };
    real.original = state;

    return real;
}
function __wbg_adapter_58(arg0, arg1, arg2) {
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hdbb4810ee474a6bf(arg0, arg1, addHeapObject(arg2));
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1);
    getUint8Memory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

let cachedUint32Memory0 = null;

function getUint32Memory0() {
    if (cachedUint32Memory0 === null || cachedUint32Memory0.byteLength === 0) {
        cachedUint32Memory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32Memory0;
}

function getArrayJsValueFromWasm0(ptr, len) {
    const mem = getUint32Memory0();
    const slice = mem.subarray(ptr / 4, ptr / 4 + len);
    const result = [];
    for (let i = 0; i < slice.length; i++) {
        result.push(takeObject(slice[i]));
    }
    return result;
}

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
    return instance.ptr;
}

let stack_pointer = 128;

function addBorrowedObject(obj) {
    if (stack_pointer == 1) throw new Error('out of js stack');
    heap[--stack_pointer] = obj;
    return stack_pointer;
}

function passArrayJsValueToWasm0(array, malloc) {
    const ptr = malloc(array.length * 4);
    const mem = getUint32Memory0();
    for (let i = 0; i < array.length; i++) {
        mem[ptr / 4 + i] = addHeapObject(array[i]);
    }
    WASM_VECTOR_LEN = array.length;
    return ptr;
}
/**
* @param {any} state_repository
* @param {DocumentsBatchTransition} transition
* @returns {Promise<void>}
*/
export function applyDocumentsBatchTransition(state_repository, transition) {
    _assertClass(transition, DocumentsBatchTransition);
    const ret = wasm.applyDocumentsBatchTransition(addHeapObject(state_repository), transition.ptr);
    return takeObject(ret);
}

/**
* @param {any} state_repository
* @param {DocumentsBatchTransition} state_transition
* @returns {Promise<ValidationResult>}
*/
export function validateDocumentsBatchTransitionState(state_repository, state_transition) {
    _assertClass(state_transition, DocumentsBatchTransition);
    const ret = wasm.validateDocumentsBatchTransitionState(addHeapObject(state_repository), state_transition.ptr);
    return takeObject(ret);
}

/**
* @param {any} state_repository
* @param {IdentityCreateTransition} state_transition
* @returns {Promise<void>}
*/
export function applyIdentityCreateTransition(state_repository, state_transition) {
    _assertClass(state_transition, IdentityCreateTransition);
    const ret = wasm.applyIdentityCreateTransition(addHeapObject(state_repository), state_transition.ptr);
    return takeObject(ret);
}

/**
* @param {any} state_repository
* @param {IdentityTopUpTransition} state_transition
* @returns {Promise<void>}
*/
export function applyIdentityTopUpTransition(state_repository, state_transition) {
    _assertClass(state_transition, IdentityTopUpTransition);
    const ret = wasm.applyIdentityTopUpTransition(addHeapObject(state_repository), state_transition.ptr);
    return takeObject(ret);
}

/**
* @param {any} state_repository
* @param {IdentityUpdateTransition} state_transition
* @returns {Promise<void>}
*/
export function applyIdentityUpdateTransition(state_repository, state_transition) {
    _assertClass(state_transition, IdentityUpdateTransition);
    const ret = wasm.applyIdentityUpdateTransition(addHeapObject(state_repository), state_transition.ptr);
    return takeObject(ret);
}

function getArrayU8FromWasm0(ptr, len) {
    return getUint8Memory0().subarray(ptr / 1, ptr / 1 + len);
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        wasm.__wbindgen_exn_store(addHeapObject(e));
    }
}
/**
* @param {Array<any>} operations
* @param {any} identity_id
* @returns {FeeResult}
*/
export function calculateStateTransitionFeeFromOperations(operations, identity_id) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.calculateStateTransitionFeeFromOperations(retptr, addHeapObject(operations), addBorrowedObject(identity_id));
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        var r2 = getInt32Memory0()[retptr / 4 + 2];
        if (r2) {
            throw takeObject(r1);
        }
        return FeeResult.__wrap(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        heap[stack_pointer++] = undefined;
    }
}

/**
* @param {any} state_repository
* @param {DataContractCreateTransition} state_transition
* @returns {Promise<ValidationResult>}
*/
export function validateDataContractCreateTransitionState(state_repository, state_transition) {
    _assertClass(state_transition, DataContractCreateTransition);
    var ptr0 = state_transition.__destroy_into_raw();
    const ret = wasm.validateDataContractCreateTransitionState(addHeapObject(state_repository), ptr0);
    return takeObject(ret);
}

/**
* @param {any} raw_parameters
* @returns {Promise<ValidationResult>}
*/
export function validateDataContractCreateTransitionBasic(raw_parameters) {
    const ret = wasm.validateDataContractCreateTransitionBasic(addHeapObject(raw_parameters));
    return takeObject(ret);
}

/**
* @param {Array<any>} js_raw_transitions
* @param {DataContract} data_contract
* @param {any} owner_id
* @returns {any[]}
*/
export function findDuplicatesByIndices(js_raw_transitions, data_contract, owner_id) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        _assertClass(data_contract, DataContract);
        wasm.findDuplicatesByIndices(retptr, addBorrowedObject(js_raw_transitions), data_contract.ptr, addBorrowedObject(owner_id));
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        var r2 = getInt32Memory0()[retptr / 4 + 2];
        var r3 = getInt32Memory0()[retptr / 4 + 3];
        if (r3) {
            throw takeObject(r2);
        }
        var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
        wasm.__wbindgen_free(r0, r1 * 4);
        return v0;
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        heap[stack_pointer++] = undefined;
        heap[stack_pointer++] = undefined;
    }
}

/**
* @param {any} js_data_contract_id
* @param {string} document_type
* @param {string} transition_action_string
* @param {Array<any>} data_triggers_list
* @returns {Array<any>}
*/
export function getDataTriggers(js_data_contract_id, document_type, transition_action_string, data_triggers_list) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(document_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(transition_action_string, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        wasm.getDataTriggers(retptr, addBorrowedObject(js_data_contract_id), ptr0, len0, ptr1, len1, addHeapObject(data_triggers_list));
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        var r2 = getInt32Memory0()[retptr / 4 + 2];
        if (r2) {
            throw takeObject(r1);
        }
        return takeObject(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        heap[stack_pointer++] = undefined;
    }
}

/**
* @returns {Array<any>}
*/
export function getAllDataTriggers() {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.getAllDataTriggers(retptr);
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        var r2 = getInt32Memory0()[retptr / 4 + 2];
        if (r2) {
            throw takeObject(r1);
        }
        return takeObject(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
* @param {any} state_repository
* @param {any} js_owner_id
* @param {Array<any>} js_document_transitions
* @param {DataContract} js_data_contract
* @param {StateTransitionExecutionContext} js_execution_context
* @returns {Promise<ValidationResult>}
*/
export function validateDocumentsUniquenessByIndices(state_repository, js_owner_id, js_document_transitions, js_data_contract, js_execution_context) {
    _assertClass(js_data_contract, DataContract);
    _assertClass(js_execution_context, StateTransitionExecutionContext);
    const ret = wasm.validateDocumentsUniquenessByIndices(addHeapObject(state_repository), addHeapObject(js_owner_id), addHeapObject(js_document_transitions), js_data_contract.ptr, js_execution_context.ptr);
    return takeObject(ret);
}

/**
* @param {any} raw_parameters
* @returns {any}
*/
export function createAssetLockProofInstance(raw_parameters) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.createAssetLockProofInstance(retptr, addHeapObject(raw_parameters));
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        var r2 = getInt32Memory0()[retptr / 4 + 2];
        if (r2) {
            throw takeObject(r1);
        }
        return takeObject(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
* @param {any} state_repository
* @param {string} raw_transaction
* @param {number} output_index
* @param {StateTransitionExecutionContext} execution_context
* @returns {Promise<ValidationResult>}
*/
export function validateAssetLockTransaction(state_repository, raw_transaction, output_index, execution_context) {
    const ptr0 = passStringToWasm0(raw_transaction, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    _assertClass(execution_context, StateTransitionExecutionContext);
    const ret = wasm.validateAssetLockTransaction(addHeapObject(state_repository), ptr0, len0, output_index, execution_context.ptr);
    return takeObject(ret);
}

/**
* @param {any} state_repository
* @param {any} raw_asset_lock_proof
* @param {StateTransitionExecutionContext} execution_context
* @returns {Promise<any>}
*/
export function fetchAssetLockTransactionOutput(state_repository, raw_asset_lock_proof, execution_context) {
    _assertClass(execution_context, StateTransitionExecutionContext);
    const ret = wasm.fetchAssetLockTransactionOutput(addHeapObject(state_repository), addHeapObject(raw_asset_lock_proof), execution_context.ptr);
    return takeObject(ret);
}

/**
* @param {any} state_repository
* @param {any} raw_asset_lock_proof
* @param {StateTransitionExecutionContext} execution_context
* @returns {Promise<any>}
*/
export function fetchAssetLockPublicKeyHash(state_repository, raw_asset_lock_proof, execution_context) {
    _assertClass(execution_context, StateTransitionExecutionContext);
    const ret = wasm.fetchAssetLockPublicKeyHash(addHeapObject(state_repository), addHeapObject(raw_asset_lock_proof), execution_context.ptr);
    return takeObject(ret);
}

/**
* @returns {any}
*/
export function generateTemporaryEcdsaPrivateKey() {
    const ret = wasm.generateTemporaryEcdsaPrivateKey();
    return takeObject(ret);
}

/**
* @param {Array<any>} js_document_transitions
* @param {DataTriggerExecutionContext} js_context
* @param {Array<any>} js_data_triggers
* @returns {Promise<Array<any>>}
*/
export function executeDataTriggers(js_document_transitions, js_context, js_data_triggers) {
    _assertClass(js_context, DataTriggerExecutionContext);
    var ptr0 = js_context.__destroy_into_raw();
    const ret = wasm.executeDataTriggers(addHeapObject(js_document_transitions), ptr0, addHeapObject(js_data_triggers));
    return takeObject(ret);
}

/**
* @param {any} contract_id
* @param {any} owner_id
* @param {string} document_type
* @param {Uint8Array} entropy
* @returns {any}
*/
export function generateDocumentId(contract_id, owner_id, document_type, entropy) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(document_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArray8ToWasm0(entropy, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        wasm.generateDocumentId(retptr, addBorrowedObject(contract_id), addBorrowedObject(owner_id), ptr0, len0, ptr1, len1);
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        var r2 = getInt32Memory0()[retptr / 4 + 2];
        if (r2) {
            throw takeObject(r1);
        }
        return takeObject(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        heap[stack_pointer++] = undefined;
        heap[stack_pointer++] = undefined;
    }
}

/**
* @param {Uint8Array} buffer
* @returns {Array<any>}
*/
export function decodeProtocolEntity(buffer) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passArray8ToWasm0(buffer, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.decodeProtocolEntity(retptr, ptr0, len0);
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        var r2 = getInt32Memory0()[retptr / 4 + 2];
        if (r2) {
            throw takeObject(r1);
        }
        return takeObject(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
* @param {any} state_repository
* @param {Array<any>} js_document_transitions
* @param {StateTransitionExecutionContext} js_execution_context
* @returns {Promise<Array<any>>}
*/
export function fetchExtendedDocuments(state_repository, js_document_transitions, js_execution_context) {
    _assertClass(js_execution_context, StateTransitionExecutionContext);
    const ret = wasm.fetchExtendedDocuments(addHeapObject(state_repository), addHeapObject(js_document_transitions), js_execution_context.ptr);
    return takeObject(ret);
}

/**
* @param {any} external_state_repository
* @param {any} js_state_transition
* @param {any} bls_adapter
* @returns {Promise<ValidationResult>}
*/
export function validateStateTransitionIdentitySignature(external_state_repository, js_state_transition, bls_adapter) {
    const ret = wasm.validateStateTransitionIdentitySignature(addHeapObject(external_state_repository), addHeapObject(js_state_transition), addHeapObject(bls_adapter));
    return takeObject(ret);
}

/**
* @param {any} state_repository
* @param {any} js_raw_document
* @returns {Promise<ValidationResult>}
*/
export function fetchAndValidateDataContract(state_repository, js_raw_document) {
    const ret = wasm.fetchAndValidateDataContract(addHeapObject(state_repository), addHeapObject(js_raw_document));
    return takeObject(ret);
}

/**
* @param {Array<any>} js_raw_transitions
* @returns {any[]}
*/
export function findDuplicatesById(js_raw_transitions) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.findDuplicatesById(retptr, addHeapObject(js_raw_transitions));
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        var r2 = getInt32Memory0()[retptr / 4 + 2];
        var r3 = getInt32Memory0()[retptr / 4 + 3];
        if (r3) {
            throw takeObject(r2);
        }
        var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
        wasm.__wbindgen_free(r0, r1 * 4);
        return v0;
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
* @param {Array<any>} js_raw_transitions
* @param {DataContract} data_contract
* @returns {ValidationResult}
*/
export function validatePartialCompoundIndices(js_raw_transitions, data_contract) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        _assertClass(data_contract, DataContract);
        wasm.validatePartialCompoundIndices(retptr, addHeapObject(js_raw_transitions), data_contract.ptr);
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        var r2 = getInt32Memory0()[retptr / 4 + 2];
        if (r2) {
            throw takeObject(r1);
        }
        return ValidationResult.__wrap(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
* @param {ProtocolVersionValidator} protocol_version_validator
* @param {any} state_repository
* @param {any} js_raw_state_transition
* @param {StateTransitionExecutionContext} execution_context
* @returns {Promise<ValidationResult>}
*/
export function validateDocumentsBatchTransitionBasic(protocol_version_validator, state_repository, js_raw_state_transition, execution_context) {
    _assertClass(protocol_version_validator, ProtocolVersionValidator);
    var ptr0 = protocol_version_validator.__destroy_into_raw();
    _assertClass(execution_context, StateTransitionExecutionContext);
    var ptr1 = execution_context.__destroy_into_raw();
    const ret = wasm.validateDocumentsBatchTransitionBasic(ptr0, addHeapObject(state_repository), addHeapObject(js_raw_state_transition), ptr1);
    return takeObject(ret);
}

/**
* @param {Uint8Array} bytes
* @returns {any}
*/
export function deserializeConsensusError(bytes) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.deserializeConsensusError(retptr, ptr0, len0);
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        var r2 = getInt32Memory0()[retptr / 4 + 2];
        if (r2) {
            throw takeObject(r1);
        }
        return takeObject(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
* @param {any} state_repository
* @param {DataContractUpdateTransition} state_transition
* @returns {Promise<ValidationResult>}
*/
export function validateDataContractUpdateTransitionState(state_repository, state_transition) {
    _assertClass(state_transition, DataContractUpdateTransition);
    var ptr0 = state_transition.__destroy_into_raw();
    const ret = wasm.validateDataContractUpdateTransitionState(addHeapObject(state_repository), ptr0);
    return takeObject(ret);
}

/**
* @param {any} old_documents_schema
* @param {any} new_documents_schema
* @returns {ValidationResult}
*/
export function validateIndicesAreBackwardCompatible(old_documents_schema, new_documents_schema) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.validateIndicesAreBackwardCompatible(retptr, addHeapObject(old_documents_schema), addHeapObject(new_documents_schema));
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        var r2 = getInt32Memory0()[retptr / 4 + 2];
        if (r2) {
            throw takeObject(r1);
        }
        return ValidationResult.__wrap(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
* @param {any} state_repository
* @param {any} raw_parameters
* @param {StateTransitionExecutionContext} execution_context
* @returns {Promise<ValidationResult>}
*/
export function validateDataContractUpdateTransitionBasic(state_repository, raw_parameters, execution_context) {
    _assertClass(execution_context, StateTransitionExecutionContext);
    const ret = wasm.validateDataContractUpdateTransitionBasic(addHeapObject(state_repository), addHeapObject(raw_parameters), execution_context.ptr);
    return takeObject(ret);
}

/**
* @param {any} state_transition_js
* @returns {FeeResult}
*/
export function calculateStateTransitionFee(state_transition_js) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.calculateStateTransitionFee(retptr, addBorrowedObject(state_transition_js));
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        var r2 = getInt32Memory0()[retptr / 4 + 2];
        if (r2) {
            throw takeObject(r1);
        }
        return FeeResult.__wrap(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        heap[stack_pointer++] = undefined;
    }
}

function passArray32ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 4);
    getUint32Memory0().set(arg, ptr / 4);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}
/**
* @param {Array<any>} operations
* @returns {DummyFeesResult}
*/
export function calculateOperationFees(operations) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.calculateOperationFees(retptr, addHeapObject(operations));
        var r0 = getInt32Memory0()[retptr / 4 + 0];
        var r1 = getInt32Memory0()[retptr / 4 + 1];
        var r2 = getInt32Memory0()[retptr / 4 + 2];
        if (r2) {
            throw takeObject(r1);
        }
        return DummyFeesResult.__wrap(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

function __wbg_adapter_1561(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen__convert__closures__invoke2_mut__hf8604d23cbf0f65f(arg0, arg1, addHeapObject(arg2), addHeapObject(arg3));
}

/**
*/
export const KeyType = Object.freeze({ ECDSA_SECP256K1:0,"0":"ECDSA_SECP256K1",BLS12_381:1,"1":"BLS12_381",ECDSA_HASH160:2,"2":"ECDSA_HASH160",BIP13_SCRIPT_HASH:3,"3":"BIP13_SCRIPT_HASH", });
/**
*/
export const KeySecurityLevel = Object.freeze({ MASTER:0,"0":"MASTER",CRITICAL:1,"1":"CRITICAL",HIGH:2,"2":"HIGH",MEDIUM:3,"3":"MEDIUM", });
/**
*/
export const KeyPurpose = Object.freeze({
/**
* at least one authentication key must be registered for all security levels
*/
AUTHENTICATION:0,"0":"AUTHENTICATION",
/**
* this key cannot be used for signing documents
*/
ENCRYPTION:1,"1":"ENCRYPTION",
/**
* this key cannot be used for signing documents
*/
DECRYPTION:2,"2":"DECRYPTION",WITHDRAW:3,"3":"WITHDRAW", });
/**
*/
export class ApplyDataContractCreateTransition {

    static __wrap(ptr) {
        const obj = Object.create(ApplyDataContractCreateTransition.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_applydatacontractcreatetransition_free(ptr);
    }
    /**
    * @param {any} state_repository
    */
    constructor(state_repository) {
        const ret = wasm.applydatacontractcreatetransition_new(addHeapObject(state_repository));
        return ApplyDataContractCreateTransition.__wrap(ret);
    }
    /**
    * @param {DataContractCreateTransition} transition
    * @returns {Promise<void>}
    */
    applyDataContractCreateTransition(transition) {
        _assertClass(transition, DataContractCreateTransition);
        var ptr0 = transition.__destroy_into_raw();
        const ret = wasm.applydatacontractcreatetransition_applyDataContractCreateTransition(this.ptr, ptr0);
        return takeObject(ret);
    }
}
/**
*/
export class ApplyDataContractUpdateTransition {

    static __wrap(ptr) {
        const obj = Object.create(ApplyDataContractUpdateTransition.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_applydatacontractupdatetransition_free(ptr);
    }
    /**
    * @param {any} state_repository
    */
    constructor(state_repository) {
        const ret = wasm.applydatacontractupdatetransition_new(addHeapObject(state_repository));
        return ApplyDataContractUpdateTransition.__wrap(ret);
    }
    /**
    * @param {DataContractUpdateTransition} transition
    * @returns {Promise<void>}
    */
    applyDataContractUpdateTransition(transition) {
        _assertClass(transition, DataContractUpdateTransition);
        var ptr0 = transition.__destroy_into_raw();
        const ret = wasm.applydatacontractupdatetransition_applyDataContractUpdateTransition(this.ptr, ptr0);
        return takeObject(ret);
    }
}
/**
*/
export class AssetLockOutputNotFoundError {

    static __wrap(ptr) {
        const obj = Object.create(AssetLockOutputNotFoundError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_assetlockoutputnotfounderror_free(ptr);
    }
}
/**
*/
export class AssetLockProof {

    static __wrap(ptr) {
        const obj = Object.create(AssetLockProof.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_assetlockproof_free(ptr);
    }
    /**
    * @param {any} raw_asset_lock_proof
    */
    constructor(raw_asset_lock_proof) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.assetlockproof_new(retptr, addHeapObject(raw_asset_lock_proof));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return AssetLockProof.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    createIdentifier() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.assetlockproof_createIdentifier(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    toObject() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.assetlockproof_toObject(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class AssetLockTransactionIsNotFoundError {

    static __wrap(ptr) {
        const obj = Object.create(AssetLockTransactionIsNotFoundError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_assetlocktransactionisnotfounderror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getTransactionId() {
        const ret = wasm.assetlocktransactionisnotfounderror_getTransactionId(this.ptr);
        return takeObject(ret);
    }
}
/**
*/
export class BalanceIsNotEnoughError {

    static __wrap(ptr) {
        const obj = Object.create(BalanceIsNotEnoughError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_balanceisnotenougherror_free(ptr);
    }
    /**
    * @param {bigint} balance
    * @param {bigint} fee
    */
    constructor(balance, fee) {
        const ret = wasm.balanceisnotenougherror_new(balance, fee);
        return BalanceIsNotEnoughError.__wrap(ret);
    }
    /**
    * @returns {bigint}
    */
    getBalance() {
        const ret = wasm.balanceisnotenougherror_getBalance(this.ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
    * @returns {bigint}
    */
    getFee() {
        const ret = wasm.balanceisnotenougherror_getFee(this.ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.balanceisnotenougherror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.balanceisnotenougherror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.balanceisnotenougherror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class ChainAssetLockProof {

    static __wrap(ptr) {
        const obj = Object.create(ChainAssetLockProof.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_chainassetlockproof_free(ptr);
    }
    /**
    * @param {any} raw_parameters
    */
    constructor(raw_parameters) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.chainassetlockproof_new(retptr, addHeapObject(raw_parameters));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return ChainAssetLockProof.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {number}
    */
    getType() {
        const ret = wasm.chainassetlockproof_getType(this.ptr);
        return ret;
    }
    /**
    * @returns {number}
    */
    getCoreChainLockedHeight() {
        const ret = wasm.chainassetlockproof_getCoreChainLockedHeight(this.ptr);
        return ret >>> 0;
    }
    /**
    * @param {number} value
    */
    setCoreChainLockedHeight(value) {
        wasm.chainassetlockproof_setCoreChainLockedHeight(this.ptr, value);
    }
    /**
    * @returns {any}
    */
    getOutPoint() {
        const ret = wasm.chainassetlockproof_getOutPoint(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {Uint8Array} out_point
    */
    setOutPoint(out_point) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArray8ToWasm0(out_point, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.chainassetlockproof_setOutPoint(retptr, this.ptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    toJSON() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.chainassetlockproof_toJSON(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    toObject() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.chainassetlockproof_toObject(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    createIdentifier() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.chainassetlockproof_createIdentifier(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class ChainAssetLockProofStructureValidator {

    static __wrap(ptr) {
        const obj = Object.create(ChainAssetLockProofStructureValidator.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_chainassetlockproofstructurevalidator_free(ptr);
    }
    /**
    * @param {any} state_repository
    */
    constructor(state_repository) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.chainassetlockproofstructurevalidator_new(retptr, addHeapObject(state_repository));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return ChainAssetLockProofStructureValidator.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} raw_asset_lock_proof
    * @param {StateTransitionExecutionContext} execution_context
    * @returns {Promise<ValidationResult>}
    */
    validate(raw_asset_lock_proof, execution_context) {
        _assertClass(execution_context, StateTransitionExecutionContext);
        const ret = wasm.chainassetlockproofstructurevalidator_validate(this.ptr, addHeapObject(raw_asset_lock_proof), execution_context.ptr);
        return takeObject(ret);
    }
}
/**
*/
export class CompatibleProtocolVersionIsNotDefinedError {

    static __wrap(ptr) {
        const obj = Object.create(CompatibleProtocolVersionIsNotDefinedError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_compatibleprotocolversionisnotdefinederror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getCurrentProtocolVersion() {
        const ret = wasm.compatibleprotocolversionisnotdefinederror_getCurrentProtocolVersion(this.ptr);
        return ret >>> 0;
    }
}
/**
*/
export class DashPlatformProtocol {

    static __wrap(ptr) {
        const obj = Object.create(DashPlatformProtocol.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_dashplatformprotocol_free(ptr);
    }
    /**
    * @param {any} bls_adapter
    * @param {any} state_repository
    * @param {any} entropy_generator
    * @param {number | undefined} maybe_protocol_version
    */
    constructor(bls_adapter, state_repository, entropy_generator, maybe_protocol_version) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.dashplatformprotocol_new(retptr, addHeapObject(bls_adapter), addHeapObject(state_repository), addHeapObject(entropy_generator), !isLikeNone(maybe_protocol_version), isLikeNone(maybe_protocol_version) ? 0 : maybe_protocol_version);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return DashPlatformProtocol.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {DataContractFacade}
    */
    get dataContract() {
        const ret = wasm.dashplatformprotocol_data_contract(this.ptr);
        return DataContractFacade.__wrap(ret);
    }
    /**
    * @returns {DocumentFacade}
    */
    get document() {
        const ret = wasm.dashplatformprotocol_document(this.ptr);
        return DocumentFacade.__wrap(ret);
    }
    /**
    * @returns {IdentityFacade}
    */
    get identity() {
        const ret = wasm.dashplatformprotocol_identity(this.ptr);
        return IdentityFacade.__wrap(ret);
    }
    /**
    * @returns {StateTransitionFacade}
    */
    get stateTransition() {
        const ret = wasm.dashplatformprotocol_state_transition(this.ptr);
        return StateTransitionFacade.__wrap(ret);
    }
    /**
    * @returns {number}
    */
    get protocolVersion() {
        const ret = wasm.dashplatformprotocol_protocol_version(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getProtocolVersion() {
        const ret = wasm.dashplatformprotocol_getProtocolVersion(this.ptr);
        return ret >>> 0;
    }
    /**
    * @param {number} protocol_version
    */
    setProtocolVersion(protocol_version) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.dashplatformprotocol_setProtocolVersion(retptr, this.ptr, protocol_version);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} state_repository
    */
    setStateRepository(state_repository) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.dashplatformprotocol_setStateRepository(retptr, this.ptr, addHeapObject(state_repository));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getStateRepository() {
        const ret = wasm.dashplatformprotocol_getStateRepository(this.ptr);
        return takeObject(ret);
    }
}
/**
*/
export class DataContract {

    static __wrap(ptr) {
        const obj = Object.create(DataContract.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datacontract_free(ptr);
    }
    /**
    * @param {any} raw_parameters
    */
    constructor(raw_parameters) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontract_new(retptr, addHeapObject(raw_parameters));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return DataContract.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {number}
    */
    getProtocolVersion() {
        const ret = wasm.datacontract_getProtocolVersion(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {any}
    */
    getId() {
        const ret = wasm.datacontract_getId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {any} id
    */
    setId(id) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontract_setId(retptr, this.ptr, addBorrowedObject(id));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
        }
    }
    /**
    * @returns {any}
    */
    getOwnerId() {
        const ret = wasm.datacontract_getOwnerId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getVersion() {
        const ret = wasm.datacontract_getVersion(this.ptr);
        return ret >>> 0;
    }
    /**
    * @param {number} v
    */
    setVersion(v) {
        wasm.datacontract_setVersion(this.ptr, v);
    }
    /**
    */
    incrementVersion() {
        wasm.datacontract_incrementVersion(this.ptr);
    }
    /**
    * @returns {string}
    */
    getJsonSchemaId() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontract_getJsonSchemaId(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @param {string} schema
    */
    setJsonMetaSchema(schema) {
        const ptr0 = passStringToWasm0(schema, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.datacontract_setJsonMetaSchema(this.ptr, ptr0, len0);
    }
    /**
    * @returns {string}
    */
    getJsonMetaSchema() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontract_getJsonMetaSchema(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @param {any} documents
    */
    setDocuments(documents) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontract_setDocuments(retptr, this.ptr, addHeapObject(documents));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getDocuments() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontract_getDocuments(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {string} doc_type
    * @returns {boolean}
    */
    isDocumentDefined(doc_type) {
        const ptr0 = passStringToWasm0(doc_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.datacontract_isDocumentDefined(this.ptr, ptr0, len0);
        return ret !== 0;
    }
    /**
    * @param {string} doc_type
    * @param {any} schema
    */
    setDocumentSchema(doc_type, schema) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passStringToWasm0(doc_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.datacontract_setDocumentSchema(retptr, this.ptr, ptr0, len0, addHeapObject(schema));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {string} doc_type
    * @returns {any}
    */
    getDocumentSchema(doc_type) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passStringToWasm0(doc_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.datacontract_getDocumentSchema(retptr, this.ptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {string} doc_type
    * @returns {any}
    */
    getDocumentSchemaRef(doc_type) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passStringToWasm0(doc_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.datacontract_getDocumentSchemaRef(retptr, this.ptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} definitions
    */
    setDefinitions(definitions) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontract_setDefinitions(retptr, this.ptr, addHeapObject(definitions));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getDefinitions() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontract_getDefinitions(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Uint8Array} e
    */
    setEntropy(e) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArray8ToWasm0(e, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.datacontract_setEntropy(retptr, this.ptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getEntropy() {
        const ret = wasm.datacontract_getEntropy(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {string} doc_type
    * @returns {any}
    */
    getBinaryProperties(doc_type) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passStringToWasm0(doc_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.datacontract_getBinaryProperties(retptr, this.ptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {Metadata | undefined}
    */
    getMetadata() {
        const ret = wasm.datacontract_getMetadata(this.ptr);
        return ret === 0 ? undefined : Metadata.__wrap(ret);
    }
    /**
    * @param {any} metadata
    */
    setMetadata(metadata) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontract_setMetadata(retptr, this.ptr, addHeapObject(metadata));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    toObject() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontract_toObject(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    toJSON() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontract_toJSON(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    toBuffer() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontract_toBuffer(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {Uint8Array}
    */
    hash() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontract_hash(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            var r3 = getInt32Memory0()[retptr / 4 + 3];
            if (r3) {
                throw takeObject(r2);
            }
            var v0 = getArrayU8FromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 1);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} v
    * @returns {DataContract}
    */
    static from(v) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontract_from(retptr, addHeapObject(v));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return DataContract.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Uint8Array} b
    * @returns {DataContract}
    */
    static fromBuffer(b) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArray8ToWasm0(b, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.datacontract_fromBuffer(retptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return DataContract.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {DataContract}
    */
    clone() {
        const ret = wasm.datacontract_clone(this.ptr);
        return DataContract.__wrap(ret);
    }
}
/**
*/
export class DataContractAlreadyPresentError {

    static __wrap(ptr) {
        const obj = Object.create(DataContractAlreadyPresentError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datacontractalreadypresenterror_free(ptr);
    }
    /**
    * @param {any} data_contract_id
    */
    constructor(data_contract_id) {
        const ret = wasm.datacontractalreadypresenterror_new(addHeapObject(data_contract_id));
        return DataContractAlreadyPresentError.__wrap(ret);
    }
    /**
    * @returns {any}
    */
    getDataContractId() {
        const ret = wasm.datacontractalreadypresenterror_getDataContractId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.datacontractalreadypresenterror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractalreadypresenterror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractalreadypresenterror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DataContractCreateTransition {

    static __wrap(ptr) {
        const obj = Object.create(DataContractCreateTransition.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datacontractcreatetransition_free(ptr);
    }
    /**
    * @param {any} raw_parameters
    */
    constructor(raw_parameters) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractcreatetransition_new(retptr, addHeapObject(raw_parameters));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return DataContractCreateTransition.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {DataContract}
    */
    getDataContract() {
        const ret = wasm.datacontractcreatetransition_getDataContract(this.ptr);
        return DataContract.__wrap(ret);
    }
    /**
    * @returns {number}
    */
    getProtocolVersion() {
        const ret = wasm.datacontractcreatetransition_getProtocolVersion(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {any}
    */
    getEntropy() {
        const ret = wasm.datacontractcreatetransition_getEntropy(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getOwnerId() {
        const ret = wasm.datacontractcreatetransition_getOwnerId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getType() {
        const ret = wasm.datacontractcreatetransition_getType(this.ptr);
        return ret >>> 0;
    }
    /**
    * @param {boolean | undefined} skip_signature
    * @returns {any}
    */
    toJSON(skip_signature) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractcreatetransition_toJSON(retptr, this.ptr, isLikeNone(skip_signature) ? 0xFFFFFF : skip_signature ? 1 : 0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {boolean | undefined} skip_signature
    * @returns {any}
    */
    toBuffer(skip_signature) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractcreatetransition_toBuffer(retptr, this.ptr, isLikeNone(skip_signature) ? 0xFFFFFF : skip_signature ? 1 : 0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any[]}
    */
    getModifiedDataIds() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractcreatetransition_getModifiedDataIds(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 4);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {boolean}
    */
    isDataContractStateTransition() {
        const ret = wasm.datacontractcreatetransition_isDataContractStateTransition(this.ptr);
        return ret !== 0;
    }
    /**
    * @returns {boolean}
    */
    isDocumentStateTransition() {
        const ret = wasm.datacontractcreatetransition_isDocumentStateTransition(this.ptr);
        return ret !== 0;
    }
    /**
    * @returns {boolean}
    */
    isIdentityStateTransition() {
        const ret = wasm.datacontractcreatetransition_isIdentityStateTransition(this.ptr);
        return ret !== 0;
    }
    /**
    * @param {StateTransitionExecutionContext} context
    */
    setExecutionContext(context) {
        _assertClass(context, StateTransitionExecutionContext);
        wasm.datacontractcreatetransition_setExecutionContext(this.ptr, context.ptr);
    }
    /**
    * @returns {StateTransitionExecutionContext}
    */
    getExecutionContext() {
        const ret = wasm.datacontractcreatetransition_getExecutionContext(this.ptr);
        return StateTransitionExecutionContext.__wrap(ret);
    }
    /**
    * @param {boolean | undefined} skip_signature
    * @returns {any}
    */
    toObject(skip_signature) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractcreatetransition_toObject(retptr, this.ptr, isLikeNone(skip_signature) ? 0xFFFFFF : skip_signature ? 1 : 0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {IdentityPublicKey} identity_public_key
    * @param {Uint8Array} private_key
    * @param {any} bls
    */
    sign(identity_public_key, private_key, bls) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(identity_public_key, IdentityPublicKey);
            const ptr0 = passArray8ToWasm0(private_key, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.datacontractcreatetransition_sign(retptr, this.ptr, identity_public_key.ptr, ptr0, len0, addHeapObject(bls));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {IdentityPublicKey} identity_public_key
    * @param {any} bls
    * @returns {boolean}
    */
    verifySignature(identity_public_key, bls) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(identity_public_key, IdentityPublicKey);
            wasm.datacontractcreatetransition_verifySignature(retptr, this.ptr, identity_public_key.ptr, addHeapObject(bls));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return r0 !== 0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DataContractDefaults {

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datacontractdefaults_free(ptr);
    }
    /**
    * @returns {string}
    */
    static get SCHEMA() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractdefaults_get_default_schema(retptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
}
/**
*/
export class DataContractFacade {

    static __wrap(ptr) {
        const obj = Object.create(DataContractFacade.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datacontractfacade_free(ptr);
    }
    /**
    * Create Data Contract
    * @param {Uint8Array} owner_id
    * @param {any} documents
    * @param {any} definitions
    * @returns {DataContract}
    */
    create(owner_id, documents, definitions) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArray8ToWasm0(owner_id, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.datacontractfacade_create(retptr, this.ptr, ptr0, len0, addHeapObject(documents), addHeapObject(definitions));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return DataContract.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * Create Data Contract from plain object
    * @param {any} js_raw_data_contract
    * @param {any} options
    * @returns {Promise<DataContract>}
    */
    createFromObject(js_raw_data_contract, options) {
        const ret = wasm.datacontractfacade_createFromObject(this.ptr, addHeapObject(js_raw_data_contract), addHeapObject(options));
        return takeObject(ret);
    }
    /**
    * Create Data Contract from buffer
    * @param {Uint8Array} buffer
    * @param {any} options
    * @returns {Promise<DataContract>}
    */
    createFromBuffer(buffer, options) {
        const ptr0 = passArray8ToWasm0(buffer, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.datacontractfacade_createFromBuffer(this.ptr, ptr0, len0, addHeapObject(options));
        return takeObject(ret);
    }
    /**
    * Create Data Contract Create State Transition
    * @param {DataContract} data_contract
    * @returns {DataContractCreateTransition}
    */
    createDataContractCreateTransition(data_contract) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(data_contract, DataContract);
            wasm.datacontractfacade_createDataContractCreateTransition(retptr, this.ptr, data_contract.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return DataContractCreateTransition.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * Create Data Contract Update State Transition
    * @param {DataContract} data_contract
    * @returns {DataContractUpdateTransition}
    */
    createDataContractUpdateTransition(data_contract) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(data_contract, DataContract);
            wasm.datacontractfacade_createDataContractUpdateTransition(retptr, this.ptr, data_contract.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return DataContractUpdateTransition.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * Validate Data Contract
    * @param {any} js_raw_data_contract
    * @returns {Promise<ValidationResult>}
    */
    validate(js_raw_data_contract) {
        const ret = wasm.datacontractfacade_validate(this.ptr, addHeapObject(js_raw_data_contract));
        return takeObject(ret);
    }
}
/**
*/
export class DataContractFactory {

    static __wrap(ptr) {
        const obj = Object.create(DataContractFactory.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datacontractfactory_free(ptr);
    }
    /**
    * @param {number} protocol_version
    * @param {DataContractValidator} validate_data_contract
    * @param {any | undefined} external_entropy_generator_arg
    */
    constructor(protocol_version, validate_data_contract, external_entropy_generator_arg) {
        _assertClass(validate_data_contract, DataContractValidator);
        var ptr0 = validate_data_contract.__destroy_into_raw();
        const ret = wasm.datacontractfactory_new(protocol_version, ptr0, isLikeNone(external_entropy_generator_arg) ? 0 : addHeapObject(external_entropy_generator_arg));
        return DataContractFactory.__wrap(ret);
    }
    /**
    * @param {Uint8Array} owner_id
    * @param {any} documents
    * @returns {DataContract}
    */
    create(owner_id, documents) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArray8ToWasm0(owner_id, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.datacontractfactory_create(retptr, this.ptr, ptr0, len0, addHeapObject(documents));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return DataContract.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} object
    * @param {boolean | undefined} skip_validation
    * @returns {Promise<DataContract>}
    */
    createFromObject(object, skip_validation) {
        const ret = wasm.datacontractfactory_createFromObject(this.ptr, addHeapObject(object), isLikeNone(skip_validation) ? 0xFFFFFF : skip_validation ? 1 : 0);
        return takeObject(ret);
    }
    /**
    * @param {Uint8Array} buffer
    * @param {boolean | undefined} skip_validation
    * @returns {Promise<DataContract>}
    */
    createFromBuffer(buffer, skip_validation) {
        const ptr0 = passArray8ToWasm0(buffer, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.datacontractfactory_createFromBuffer(this.ptr, ptr0, len0, isLikeNone(skip_validation) ? 0xFFFFFF : skip_validation ? 1 : 0);
        return takeObject(ret);
    }
    /**
    * @param {DataContract} data_contract
    * @returns {Promise<DataContractCreateTransition>}
    */
    createDataContractCreateTransition(data_contract) {
        _assertClass(data_contract, DataContract);
        const ret = wasm.datacontractfactory_createDataContractCreateTransition(this.ptr, data_contract.ptr);
        return takeObject(ret);
    }
}
/**
*/
export class DataContractGenericError {

    static __wrap(ptr) {
        const obj = Object.create(DataContractGenericError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datacontractgenericerror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getMessage() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractgenericerror_getMessage(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
}
/**
*/
export class DataContractHaveNewUniqueIndexError {

    static __wrap(ptr) {
        const obj = Object.create(DataContractHaveNewUniqueIndexError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datacontracthavenewuniqueindexerror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getDocumentType() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontracthavenewuniqueindexerror_getDocumentType(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getIndexName() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontracthavenewuniqueindexerror_getIndexName(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.datacontracthavenewuniqueindexerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontracthavenewuniqueindexerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontracthavenewuniqueindexerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DataContractImmutablePropertiesUpdateError {

    static __wrap(ptr) {
        const obj = Object.create(DataContractImmutablePropertiesUpdateError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datacontractimmutablepropertiesupdateerror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getOperation() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractimmutablepropertiesupdateerror_getOperation(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getFieldPath() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractimmutablepropertiesupdateerror_getFieldPath(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.datacontractimmutablepropertiesupdateerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractimmutablepropertiesupdateerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractimmutablepropertiesupdateerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DataContractInvalidIndexDefinitionUpdateError {

    static __wrap(ptr) {
        const obj = Object.create(DataContractInvalidIndexDefinitionUpdateError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datacontractinvalidindexdefinitionupdateerror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getDocumentType() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractinvalidindexdefinitionupdateerror_getDocumentType(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getIndexName() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractinvalidindexdefinitionupdateerror_getIndexName(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.datacontractinvalidindexdefinitionupdateerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractinvalidindexdefinitionupdateerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractinvalidindexdefinitionupdateerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}

export class DataContractMaxDepthError {

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datacontractmaxdeptherror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getMaxDepth() {
        const ret = wasm.datacontractmaxdeptherror_getMaxDepth(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getSchemaDepth() {
        const ret = wasm.datacontractmaxdeptherror_getSchemaDepth(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.datacontractmaxdeptherror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractmaxdeptherror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractmaxdeptherror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DataContractMaxDepthExceedError {

    static __wrap(ptr) {
        const obj = Object.create(DataContractMaxDepthExceedError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datacontractmaxdepthexceederror_free(ptr);
    }
}
/**
*/
export class DataContractNotPresentError {

    static __wrap(ptr) {
        const obj = Object.create(DataContractNotPresentError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datacontractnotpresenterror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getDataContractId() {
        const ret = wasm.datacontractnotpresenterror_getDataContractId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.datacontractnotpresenterror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractnotpresenterror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractnotpresenterror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DataContractNotPresentNotConsensusError {

    static __wrap(ptr) {
        const obj = Object.create(DataContractNotPresentNotConsensusError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datacontractnotpresentnotconsensuserror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getDataContractId() {
        const ret = wasm.datacontractnotpresentnotconsensuserror_getDataContractId(this.ptr);
        return takeObject(ret);
    }
}
/**
*/
export class DataContractUniqueIndicesChangedError {

    static __wrap(ptr) {
        const obj = Object.create(DataContractUniqueIndicesChangedError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datacontractuniqueindiceschangederror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getDocumentType() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractuniqueindiceschangederror_getDocumentType(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getIndexName() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractuniqueindiceschangederror_getIndexName(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.datacontractuniqueindiceschangederror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractuniqueindiceschangederror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractuniqueindiceschangederror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DataContractUpdateTransition {

    static __wrap(ptr) {
        const obj = Object.create(DataContractUpdateTransition.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datacontractupdatetransition_free(ptr);
    }
    /**
    * @param {any} raw_parameters
    */
    constructor(raw_parameters) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractupdatetransition_new(retptr, addHeapObject(raw_parameters));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return DataContractUpdateTransition.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {DataContract}
    */
    getDataContract() {
        const ret = wasm.datacontractupdatetransition_getDataContract(this.ptr);
        return DataContract.__wrap(ret);
    }
    /**
    * @returns {number}
    */
    getProtocolVersion() {
        const ret = wasm.datacontractupdatetransition_getProtocolVersion(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {any}
    */
    getEntropy() {
        const ret = wasm.datacontractupdatetransition_getEntropy(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getOwnerId() {
        const ret = wasm.datacontractupdatetransition_getOwnerId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getType() {
        const ret = wasm.datacontractupdatetransition_getType(this.ptr);
        return ret >>> 0;
    }
    /**
    * @param {boolean | undefined} skip_signature
    * @returns {any}
    */
    toJSON(skip_signature) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractupdatetransition_toJSON(retptr, this.ptr, isLikeNone(skip_signature) ? 0xFFFFFF : skip_signature ? 1 : 0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {boolean | undefined} skip_signature
    * @returns {any}
    */
    toBuffer(skip_signature) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractupdatetransition_toBuffer(retptr, this.ptr, isLikeNone(skip_signature) ? 0xFFFFFF : skip_signature ? 1 : 0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any[]}
    */
    getModifiedDataIds() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractupdatetransition_getModifiedDataIds(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 4);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {boolean}
    */
    isDataContractStateTransition() {
        const ret = wasm.datacontractupdatetransition_isDataContractStateTransition(this.ptr);
        return ret !== 0;
    }
    /**
    * @returns {boolean}
    */
    isDocumentStateTransition() {
        const ret = wasm.datacontractupdatetransition_isDocumentStateTransition(this.ptr);
        return ret !== 0;
    }
    /**
    * @returns {boolean}
    */
    isIdentityStateTransition() {
        const ret = wasm.datacontractupdatetransition_isIdentityStateTransition(this.ptr);
        return ret !== 0;
    }
    /**
    * @param {StateTransitionExecutionContext} context
    */
    setExecutionContext(context) {
        _assertClass(context, StateTransitionExecutionContext);
        wasm.datacontractupdatetransition_setExecutionContext(this.ptr, context.ptr);
    }
    /**
    * @returns {StateTransitionExecutionContext}
    */
    getExecutionContext() {
        const ret = wasm.datacontractupdatetransition_getExecutionContext(this.ptr);
        return StateTransitionExecutionContext.__wrap(ret);
    }
    /**
    * @param {boolean | undefined} skip_signature
    * @returns {any}
    */
    hash(skip_signature) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractupdatetransition_hash(retptr, this.ptr, isLikeNone(skip_signature) ? 0xFFFFFF : skip_signature ? 1 : 0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {boolean | undefined} skip_signature
    * @returns {any}
    */
    toObject(skip_signature) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractupdatetransition_toObject(retptr, this.ptr, isLikeNone(skip_signature) ? 0xFFFFFF : skip_signature ? 1 : 0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {IdentityPublicKey} identity_public_key
    * @param {Uint8Array} private_key
    * @param {any} bls
    */
    sign(identity_public_key, private_key, bls) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(identity_public_key, IdentityPublicKey);
            const ptr0 = passArray8ToWasm0(private_key, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.datacontractupdatetransition_sign(retptr, this.ptr, identity_public_key.ptr, ptr0, len0, addHeapObject(bls));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {IdentityPublicKey} identity_public_key
    * @param {any} bls
    * @returns {boolean}
    */
    verifySignature(identity_public_key, bls) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(identity_public_key, IdentityPublicKey);
            wasm.datacontractupdatetransition_verifySignature(retptr, this.ptr, identity_public_key.ptr, addHeapObject(bls));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return r0 !== 0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DataContractValidator {

    static __wrap(ptr) {
        const obj = Object.create(DataContractValidator.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datacontractvalidator_free(ptr);
    }
    /**
    */
    constructor() {
        const ret = wasm.datacontractvalidator_new();
        return DataContractValidator.__wrap(ret);
    }
    /**
    * @param {any} raw_data_contract
    * @returns {ValidationResult}
    */
    validate(raw_data_contract) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datacontractvalidator_validate(retptr, this.ptr, addHeapObject(raw_data_contract));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return ValidationResult.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DataTrigger {

    static __wrap(ptr) {
        const obj = Object.create(DataTrigger.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datatrigger_free(ptr);
    }
    /**
    * @returns {any}
    */
    get dataContractId() {
        const ret = wasm.datatrigger_get_data_contract_id(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {any} id
    */
    set dataContractId(id) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datatrigger_set_data_contract_id(retptr, this.ptr, addBorrowedObject(id));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
        }
    }
    /**
    * @returns {string}
    */
    get documentType() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datatrigger_get_document_type(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @param {string} id
    */
    set documentType(id) {
        const ptr0 = passStringToWasm0(id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.datatrigger_set_document_type(this.ptr, ptr0, len0);
    }
    /**
    * @returns {string}
    */
    get dataTriggerKind() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datatrigger_get_data_trigger_kind(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    get transitionAction() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datatrigger_get_transition_action(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @param {string} action_string
    */
    set transitionAction(action_string) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passStringToWasm0(action_string, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.datatrigger_set_transition_action(retptr, this.ptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    get topLevelIdentity() {
        const ret = wasm.datatrigger_get_top_level_identity(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {any} maybe_id
    */
    set topLevelIdentity(maybe_id) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datatrigger_set_top_level_identity(retptr, this.ptr, addBorrowedObject(maybe_id));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
        }
    }
}
/**
*/
export class DataTriggerConditionError {

    static __wrap(ptr) {
        const obj = Object.create(DataTriggerConditionError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datatriggerconditionerror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getDataContractId() {
        const ret = wasm.datatriggerconditionerror_getDataContractId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getDocumentId() {
        const ret = wasm.datatriggerconditionerror_getDocumentId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {string}
    */
    getMessage() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datatriggerconditionerror_getMessage(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.datatriggerconditionerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datatriggerconditionerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datatriggerconditionerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DataTriggerExecutionContext {

    static __wrap(ptr) {
        const obj = Object.create(DataTriggerExecutionContext.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datatriggerexecutioncontext_free(ptr);
    }
    /**
    * @param {any} state_repository
    * @param {any} js_owner_id
    * @param {DataContract} data_contract
    * @param {StateTransitionExecutionContext} state_transition_execution_context
    */
    constructor(state_repository, js_owner_id, data_contract, state_transition_execution_context) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(data_contract, DataContract);
            var ptr0 = data_contract.__destroy_into_raw();
            _assertClass(state_transition_execution_context, StateTransitionExecutionContext);
            var ptr1 = state_transition_execution_context.__destroy_into_raw();
            wasm.datatriggerexecutioncontext_new(retptr, addHeapObject(state_repository), addBorrowedObject(js_owner_id), ptr0, ptr1);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return DataTriggerExecutionContext.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
        }
    }
    /**
    * @returns {any}
    */
    get ownerId() {
        const ret = wasm.datatriggerexecutioncontext_ownerId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {any} owner_id
    */
    set ownerId(owner_id) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datatriggerexecutioncontext_set_ownerId(retptr, this.ptr, addBorrowedObject(owner_id));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
        }
    }
    /**
    * @returns {DataContract}
    */
    get dataContract() {
        const ret = wasm.datatriggerexecutioncontext_dataContract(this.ptr);
        return DataContract.__wrap(ret);
    }
    /**
    * @param {DataContract} data_contract
    */
    set dataContract(data_contract) {
        _assertClass(data_contract, DataContract);
        wasm.datatriggerexecutioncontext_set_dataContract(this.ptr, data_contract.ptr);
    }
    /**
    * @returns {StateTransitionExecutionContext}
    */
    get stateTransitionExecutionContext() {
        const ret = wasm.datatriggerexecutioncontext_stateTransitionExecutionContext(this.ptr);
        return StateTransitionExecutionContext.__wrap(ret);
    }
    /**
    * @param {StateTransitionExecutionContext} context
    */
    set statTransitionExecutionContext(context) {
        _assertClass(context, StateTransitionExecutionContext);
        wasm.datatriggerexecutioncontext_set_statTransitionExecutionContext(this.ptr, context.ptr);
    }
}
/**
*/
export class DataTriggerExecutionError {

    static __wrap(ptr) {
        const obj = Object.create(DataTriggerExecutionError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datatriggerexecutionerror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getDataContractId() {
        const ret = wasm.datatriggerexecutionerror_getDataContractId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getDocumentId() {
        const ret = wasm.datatriggerexecutionerror_getDocumentId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {string}
    */
    getMessage() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datatriggerexecutionerror_getMessage(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.datatriggerexecutionerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datatriggerexecutionerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datatriggerexecutionerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DataTriggerExecutionResult {

    static __wrap(ptr) {
        const obj = Object.create(DataTriggerExecutionResult.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datatriggerexecutionresult_free(ptr);
    }
    /**
    * @returns {boolean}
    */
    isOk() {
        const ret = wasm.datatriggerexecutionresult_isOk(this.ptr);
        return ret !== 0;
    }
    /**
    * @returns {Array<any>}
    */
    getErrors() {
        const ret = wasm.datatriggerexecutionresult_getErrors(this.ptr);
        return takeObject(ret);
    }
}
/**
*/
export class DataTriggerInvalidResultError {

    static __wrap(ptr) {
        const obj = Object.create(DataTriggerInvalidResultError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_datatriggerinvalidresulterror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getDataContractId() {
        const ret = wasm.datatriggerinvalidresulterror_getDataContractId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getDocumentId() {
        const ret = wasm.datatriggerinvalidresulterror_getDocumentId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.datatriggerinvalidresulterror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datatriggerinvalidresulterror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.datatriggerinvalidresulterror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class Document {

    static __wrap(ptr) {
        const obj = Object.create(Document.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_document_free(ptr);
    }
    /**
    * @param {any} js_raw_document
    * @param {DataContract} js_data_contract
    * @param {any} js_document_type_name
    */
    constructor(js_raw_document, js_data_contract, js_document_type_name) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(js_data_contract, DataContract);
            wasm.document_new(retptr, addHeapObject(js_raw_document), js_data_contract.ptr, addHeapObject(js_document_type_name));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return Document.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getId() {
        const ret = wasm.document_getId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {any} js_id
    */
    setId(js_id) {
        wasm.document_setId(this.ptr, addHeapObject(js_id));
    }
    /**
    * @param {any} owner_id
    */
    setOwnerId(owner_id) {
        wasm.document_setOwnerId(this.ptr, addHeapObject(owner_id));
    }
    /**
    * @returns {any}
    */
    getOwnerId() {
        const ret = wasm.document_getOwnerId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {number | undefined} revision
    */
    setRevision(revision) {
        wasm.document_setRevision(this.ptr, !isLikeNone(revision), isLikeNone(revision) ? 0 : revision);
    }
    /**
    * @returns {number | undefined}
    */
    getRevision() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.document_getRevision(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return r0 === 0 ? undefined : r1 >>> 0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} d
    */
    setData(d) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.document_setData(retptr, this.ptr, addHeapObject(d));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getData() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.document_getData(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {string} path
    * @param {any} js_value_to_set
    */
    set(path, js_value_to_set) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passStringToWasm0(path, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.document_set(retptr, this.ptr, ptr0, len0, addHeapObject(js_value_to_set));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {string} path
    * @returns {any}
    */
    get(path) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passStringToWasm0(path, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.document_get(retptr, this.ptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Date | undefined} created_at
    */
    setCreatedAt(created_at) {
        wasm.document_setCreatedAt(this.ptr, isLikeNone(created_at) ? 0 : addHeapObject(created_at));
    }
    /**
    * @param {Date | undefined} updated_at
    */
    setUpdatedAt(updated_at) {
        wasm.document_setUpdatedAt(this.ptr, isLikeNone(updated_at) ? 0 : addHeapObject(updated_at));
    }
    /**
    * @returns {Date | undefined}
    */
    getCreatedAt() {
        const ret = wasm.document_getCreatedAt(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {Date | undefined}
    */
    getUpdatedAt() {
        const ret = wasm.document_getUpdatedAt(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {any} options
    * @param {DataContract} data_contract
    * @param {string} document_type_name
    * @returns {any}
    */
    toObject(options, data_contract, document_type_name) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(data_contract, DataContract);
            const ptr0 = passStringToWasm0(document_type_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.document_toObject(retptr, this.ptr, addBorrowedObject(options), data_contract.ptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
        }
    }
    /**
    * @returns {any}
    */
    toJSON() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.document_toJSON(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    toBuffer() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.document_toBuffer(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {DataContract} data_contract
    * @param {string} document_type_name
    * @returns {any}
    */
    hash(data_contract, document_type_name) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(data_contract, DataContract);
            var ptr0 = data_contract.__destroy_into_raw();
            const ptr1 = passStringToWasm0(document_type_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            wasm.document_hash(retptr, this.ptr, ptr0, ptr1, len1);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {Document}
    */
    clone() {
        const ret = wasm.document_clone(this.ptr);
        return Document.__wrap(ret);
    }
}
/**
*/
export class DocumentAlreadyExistsError {

    static __wrap(ptr) {
        const obj = Object.create(DocumentAlreadyExistsError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_documentalreadyexistserror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getDocumentTransition() {
        const ret = wasm.documentalreadyexistserror_getDocumentTransition(this.ptr);
        return takeObject(ret);
    }
}
/**
*/
export class DocumentAlreadyPresentError {

    static __wrap(ptr) {
        const obj = Object.create(DocumentAlreadyPresentError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_documentalreadypresenterror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getDocumentId() {
        const ret = wasm.documentalreadypresenterror_getDocumentId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.documentalreadypresenterror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentalreadypresenterror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentalreadypresenterror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DocumentCreateTransition {

    static __wrap(ptr) {
        const obj = Object.create(DocumentCreateTransition.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_documentcreatetransition_free(ptr);
    }
    /**
    * @param {any} raw_object
    * @param {DataContract} data_contract
    */
    constructor(raw_object, data_contract) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(data_contract, DataContract);
            wasm.documentcreatetransition_from_raw_object(retptr, addHeapObject(raw_object), data_contract.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return DocumentCreateTransition.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {Uint8Array}
    */
    getEntropy() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentcreatetransition_getEntropy(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayU8FromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 1);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {Date | undefined}
    */
    getCreatedAt() {
        const ret = wasm.documentcreatetransition_getCreatedAt(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {Date | undefined}
    */
    getUpdatedAt() {
        const ret = wasm.documentcreatetransition_getUpdatedAt(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {bigint}
    */
    getRevision() {
        const ret = wasm.documentcreatetransition_getRevision(this.ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
    * @returns {any}
    */
    getId() {
        const ret = wasm.documentcreatetransition_getId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {string}
    */
    getType() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentcreatetransition_getType(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getAction() {
        const ret = wasm.documentcreatetransition_getAction(this.ptr);
        return ret;
    }
    /**
    * @returns {DataContract}
    */
    getDataContract() {
        const ret = wasm.documentcreatetransition_getDataContract(this.ptr);
        return DataContract.__wrap(ret);
    }
    /**
    * @returns {any}
    */
    getDataContractId() {
        const ret = wasm.documentcreatetransition_getDataContractId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {string} path
    * @returns {any}
    */
    get(path) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passStringToWasm0(path, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.documentcreatetransition_get(retptr, this.ptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} options
    * @returns {any}
    */
    toObject(options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentcreatetransition_toObject(retptr, this.ptr, addBorrowedObject(options));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
        }
    }
    /**
    * @returns {any}
    */
    toJSON() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentcreatetransition_toJSON(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getData() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentcreatetransition_getData(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DocumentDeleteTransition {

    static __wrap(ptr) {
        const obj = Object.create(DocumentDeleteTransition.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_documentdeletetransition_free(ptr);
    }
    /**
    * @returns {number}
    */
    getAction() {
        const ret = wasm.documentdeletetransition_getAction(this.ptr);
        return ret;
    }
    /**
    * @param {any} options
    * @returns {any}
    */
    toObject(options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentdeletetransition_toObject(retptr, this.ptr, addBorrowedObject(options));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
        }
    }
    /**
    * @returns {any}
    */
    toJSON() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentdeletetransition_toJSON(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getId() {
        const ret = wasm.documentdeletetransition_getId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {string}
    */
    getType() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentdeletetransition_getType(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {DataContract}
    */
    getDataContract() {
        const ret = wasm.documentdeletetransition_getDataContract(this.ptr);
        return DataContract.__wrap(ret);
    }
    /**
    * @returns {any}
    */
    getDataContractId() {
        const ret = wasm.documentdeletetransition_getDataContractId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {string} path
    * @returns {any}
    */
    get(path) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passStringToWasm0(path, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.documentdeletetransition_get(retptr, this.ptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DocumentFacade {

    static __wrap(ptr) {
        const obj = Object.create(DocumentFacade.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_documentfacade_free(ptr);
    }
    /**
    * @param {DocumentValidator} document_validator
    * @param {DocumentFactory} document_factory
    * @param {FetchAndValidateDataContractFactory} data_contract_fetcher_and_validator
    */
    constructor(document_validator, document_factory, data_contract_fetcher_and_validator) {
        _assertClass(document_validator, DocumentValidator);
        var ptr0 = document_validator.__destroy_into_raw();
        _assertClass(document_factory, DocumentFactory);
        var ptr1 = document_factory.__destroy_into_raw();
        _assertClass(data_contract_fetcher_and_validator, FetchAndValidateDataContractFactory);
        var ptr2 = data_contract_fetcher_and_validator.__destroy_into_raw();
        const ret = wasm.documentfacade_new(ptr0, ptr1, ptr2);
        return DocumentFacade.__wrap(ret);
    }
    /**
    * @param {DataContract} data_contract
    * @param {any} js_owner_id
    * @param {string} document_type
    * @param {any} data
    * @returns {ExtendedDocument}
    */
    create(data_contract, js_owner_id, document_type, data) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(data_contract, DataContract);
            const ptr0 = passStringToWasm0(document_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.documentfacade_create(retptr, this.ptr, data_contract.ptr, addBorrowedObject(js_owner_id), ptr0, len0, addBorrowedObject(data));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return ExtendedDocument.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
            heap[stack_pointer++] = undefined;
        }
    }
    /**
    * Creates Document from object
    * @param {any} raw_document
    * @param {any} options
    * @returns {Promise<ExtendedDocument>}
    */
    createFromObject(raw_document, options) {
        const ret = wasm.documentfacade_createFromObject(this.ptr, addHeapObject(raw_document), addHeapObject(options));
        return takeObject(ret);
    }
    /**
    * Creates Document form bytes
    * @param {Uint8Array} bytes
    * @param {any} options
    * @returns {Promise<ExtendedDocument>}
    */
    createFromBuffer(bytes, options) {
        const ptr0 = passArray8ToWasm0(bytes, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.documentfacade_createFromBuffer(this.ptr, ptr0, len0, addHeapObject(options));
        return takeObject(ret);
    }
    /**
    * Creates Documents State Transition
    * @param {any} documents
    * @returns {DocumentsBatchTransition}
    */
    createStateTransition(documents) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentfacade_createStateTransition(retptr, this.ptr, addBorrowedObject(documents));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return DocumentsBatchTransition.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
        }
    }
    /**
    * Creates Documents State Transition
    * @param {any} document
    * @returns {Promise<ValidationResult>}
    */
    validate(document) {
        const ret = wasm.documentfacade_validate(this.ptr, addHeapObject(document));
        return takeObject(ret);
    }
    /**
    * Creates Documents State Transition
    * @param {any} js_raw_document
    * @returns {Promise<ValidationResult>}
    */
    validate_raw_document(js_raw_document) {
        const ret = wasm.documentfacade_validate_raw_document(this.ptr, addHeapObject(js_raw_document));
        return takeObject(ret);
    }
}
/**
*/
export class DocumentFactory {

    static __wrap(ptr) {
        const obj = Object.create(DocumentFactory.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_documentfactory_free(ptr);
    }
    /**
    * @param {number} protocol_version
    * @param {DocumentValidator} document_validator
    * @param {any} state_repository
    * @param {any | undefined} external_entropy_generator_arg
    */
    constructor(protocol_version, document_validator, state_repository, external_entropy_generator_arg) {
        _assertClass(document_validator, DocumentValidator);
        var ptr0 = document_validator.__destroy_into_raw();
        const ret = wasm.documentfactory_new(protocol_version, ptr0, addHeapObject(state_repository), isLikeNone(external_entropy_generator_arg) ? 0 : addHeapObject(external_entropy_generator_arg));
        return DocumentFactory.__wrap(ret);
    }
    /**
    * @param {DataContract} data_contract
    * @param {any} js_owner_id
    * @param {string} document_type
    * @param {any} data
    * @returns {ExtendedDocument}
    */
    create(data_contract, js_owner_id, document_type, data) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(data_contract, DataContract);
            const ptr0 = passStringToWasm0(document_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.documentfactory_create(retptr, this.ptr, data_contract.ptr, addBorrowedObject(js_owner_id), ptr0, len0, addBorrowedObject(data));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return ExtendedDocument.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
            heap[stack_pointer++] = undefined;
        }
    }
    /**
    * @param {any} documents
    * @returns {DocumentsBatchTransition}
    */
    createStateTransition(documents) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentfactory_createStateTransition(retptr, this.ptr, addBorrowedObject(documents));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return DocumentsBatchTransition.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
        }
    }
    /**
    * @param {any} raw_document_js
    * @param {any} options
    * @returns {Promise<ExtendedDocument>}
    */
    createFromObject(raw_document_js, options) {
        const ret = wasm.documentfactory_createFromObject(this.ptr, addHeapObject(raw_document_js), addHeapObject(options));
        return takeObject(ret);
    }
    /**
    * @param {Uint8Array} buffer
    * @param {any} options
    * @returns {Promise<ExtendedDocument>}
    */
    createFromBuffer(buffer, options) {
        const ptr0 = passArray8ToWasm0(buffer, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.documentfactory_createFromBuffer(this.ptr, ptr0, len0, addHeapObject(options));
        return takeObject(ret);
    }
}
/**
*/
export class DocumentNoRevisionError {

    static __wrap(ptr) {
        const obj = Object.create(DocumentNoRevisionError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_documentnorevisionerror_free(ptr);
    }
    /**
    * @param {Document} document
    */
    constructor(document) {
        _assertClass(document, Document);
        var ptr0 = document.__destroy_into_raw();
        const ret = wasm.documentnorevisionerror_new(ptr0);
        return DocumentNoRevisionError.__wrap(ret);
    }
    /**
    * @returns {Document}
    */
    getDocument() {
        const ret = wasm.documentnorevisionerror_getDocument(this.ptr);
        return Document.__wrap(ret);
    }
}
/**
*/
export class DocumentNotFoundError {

    static __wrap(ptr) {
        const obj = Object.create(DocumentNotFoundError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_documentnotfounderror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getDocumentId() {
        const ret = wasm.documentnotfounderror_getDocumentId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.documentnotfounderror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentnotfounderror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentnotfounderror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DocumentNotProvidedError {

    static __wrap(ptr) {
        const obj = Object.create(DocumentNotProvidedError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_documentnotprovidederror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getDocumentTransition() {
        const ret = wasm.documentnotprovidederror_getDocumentTransition(this.ptr);
        return takeObject(ret);
    }
}
/**
*/
export class DocumentOwnerIdMismatchError {

    static __wrap(ptr) {
        const obj = Object.create(DocumentOwnerIdMismatchError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_documentowneridmismatcherror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getDocumentId() {
        const ret = wasm.documentowneridmismatcherror_getDocumentId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getDocumentOwnerId() {
        const ret = wasm.documentowneridmismatcherror_getDocumentOwnerId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getExistingDocumentOwnerId() {
        const ret = wasm.documentowneridmismatcherror_getExistingDocumentOwnerId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.documentowneridmismatcherror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentowneridmismatcherror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentowneridmismatcherror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DocumentReplaceTransition {

    static __wrap(ptr) {
        const obj = Object.create(DocumentReplaceTransition.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_documentreplacetransition_free(ptr);
    }
    /**
    * @param {any} raw_object
    * @param {DataContract} data_contract
    */
    constructor(raw_object, data_contract) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(data_contract, DataContract);
            wasm.documentreplacetransition_from_raw_object(retptr, addHeapObject(raw_object), data_contract.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return DocumentReplaceTransition.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {number}
    */
    getAction() {
        const ret = wasm.documentreplacetransition_getAction(this.ptr);
        return ret;
    }
    /**
    * @returns {bigint}
    */
    getRevision() {
        const ret = wasm.documentreplacetransition_getRevision(this.ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
    * @returns {Date | undefined}
    */
    getUpdatedAt() {
        const ret = wasm.documentreplacetransition_getUpdatedAt(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {any} options
    * @returns {any}
    */
    toObject(options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentreplacetransition_toObject(retptr, this.ptr, addBorrowedObject(options));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
        }
    }
    /**
    * @returns {any}
    */
    toJSON() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentreplacetransition_toJSON(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getData() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentreplacetransition_getData(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getId() {
        const ret = wasm.documentreplacetransition_getId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {string}
    */
    getType() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentreplacetransition_getType(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {DataContract}
    */
    getDataContract() {
        const ret = wasm.documentreplacetransition_getDataContract(this.ptr);
        return DataContract.__wrap(ret);
    }
    /**
    * @returns {any}
    */
    getDataContractId() {
        const ret = wasm.documentreplacetransition_getDataContractId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {string} path
    * @returns {any}
    */
    get(path) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passStringToWasm0(path, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.documentreplacetransition_get(retptr, this.ptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DocumentTimestampWindowViolationError {

    static __wrap(ptr) {
        const obj = Object.create(DocumentTimestampWindowViolationError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_documenttimestampwindowviolationerror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getDocumentId() {
        const ret = wasm.documenttimestampwindowviolationerror_getDocumentId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {string}
    */
    getTimestampName() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documenttimestampwindowviolationerror_getTimestampName(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {Date}
    */
    getTimestamp() {
        const ret = wasm.documenttimestampwindowviolationerror_getTimestamp(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {Date}
    */
    getTimeWindowStart() {
        const ret = wasm.documenttimestampwindowviolationerror_getTimeWindowStart(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {Date}
    */
    getTimeWindowEnd() {
        const ret = wasm.documenttimestampwindowviolationerror_getTimeWindowEnd(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.documenttimestampwindowviolationerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documenttimestampwindowviolationerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documenttimestampwindowviolationerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DocumentTimestampsMismatchError {

    static __wrap(ptr) {
        const obj = Object.create(DocumentTimestampsMismatchError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_documenttimestampsmismatcherror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getDocumentId() {
        const ret = wasm.documenttimestampsmismatcherror_getDocumentId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.documenttimestampsmismatcherror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documenttimestampsmismatcherror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documenttimestampsmismatcherror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DocumentTransition {

    static __wrap(ptr) {
        const obj = Object.create(DocumentTransition.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_documenttransition_free(ptr);
    }
    /**
    * @returns {any}
    */
    getId() {
        const ret = wasm.documenttransition_getId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {string}
    */
    getType() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documenttransition_getType(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getAction() {
        const ret = wasm.documenttransition_getAction(this.ptr);
        return ret;
    }
    /**
    * @returns {DataContract}
    */
    getDataContract() {
        const ret = wasm.documenttransition_getDataContract(this.ptr);
        return DataContract.__wrap(ret);
    }
    /**
    * @returns {any}
    */
    getDataContractId() {
        const ret = wasm.documenttransition_getDataContractId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {any} js_data_contract_id
    */
    setDataContractId(js_data_contract_id) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documenttransition_setDataContractId(retptr, this.ptr, addBorrowedObject(js_data_contract_id));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
        }
    }
    /**
    * @returns {any}
    */
    getRevision() {
        const ret = wasm.documenttransition_getRevision(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getCreatedAt() {
        const ret = wasm.documenttransition_getCreatedAt(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getUpdatedAt() {
        const ret = wasm.documenttransition_getUpdatedAt(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {Date | undefined} updated_at
    */
    setUpdatedAt(updated_at) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documenttransition_setUpdatedAt(retptr, this.ptr, isLikeNone(updated_at) ? 0 : addHeapObject(updated_at));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Date | undefined} created_at
    */
    setCreatedAt(created_at) {
        wasm.documenttransition_setCreatedAt(this.ptr, isLikeNone(created_at) ? 0 : addHeapObject(created_at));
    }
    /**
    * @returns {any}
    */
    getData() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documenttransition_getData(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {string} path
    * @returns {any}
    */
    get(path) {
        const ptr0 = passStringToWasm0(path, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.documenttransition_get(this.ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
    * @param {any} options
    * @returns {any}
    */
    toObject(options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documenttransition_toObject(retptr, this.ptr, addBorrowedObject(options));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
        }
    }
    /**
    * @returns {any}
    */
    toJSON() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documenttransition_toJSON(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {DocumentCreateTransition} js_create_transition
    * @returns {DocumentTransition}
    */
    static fromTransitionCreate(js_create_transition) {
        _assertClass(js_create_transition, DocumentCreateTransition);
        var ptr0 = js_create_transition.__destroy_into_raw();
        const ret = wasm.documenttransition_fromTransitionCreate(ptr0);
        return DocumentTransition.__wrap(ret);
    }
    /**
    * @param {DocumentReplaceTransition} js_replace_transition
    * @returns {DocumentTransition}
    */
    static fromTransitionReplace(js_replace_transition) {
        _assertClass(js_replace_transition, DocumentReplaceTransition);
        var ptr0 = js_replace_transition.__destroy_into_raw();
        const ret = wasm.documenttransition_fromTransitionReplace(ptr0);
        return DocumentTransition.__wrap(ret);
    }
    /**
    * @param {DocumentDeleteTransition} js_delete_transition
    * @returns {DocumentTransition}
    */
    static fromTransitionDelete(js_delete_transition) {
        _assertClass(js_delete_transition, DocumentDeleteTransition);
        var ptr0 = js_delete_transition.__destroy_into_raw();
        const ret = wasm.documenttransition_fromTransitionDelete(ptr0);
        return DocumentTransition.__wrap(ret);
    }
}
/**
*/
export class DocumentTransitions {

    static __wrap(ptr) {
        const obj = Object.create(DocumentTransitions.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_documenttransitions_free(ptr);
    }
    /**
    */
    constructor() {
        const ret = wasm.documenttransitions_new();
        return DocumentTransitions.__wrap(ret);
    }
    /**
    * @param {ExtendedDocument} transition
    */
    addTransitionCreate(transition) {
        _assertClass(transition, ExtendedDocument);
        var ptr0 = transition.__destroy_into_raw();
        wasm.documenttransitions_addTransitionCreate(this.ptr, ptr0);
    }
    /**
    * @param {ExtendedDocument} transition
    */
    addTransitionReplace(transition) {
        _assertClass(transition, ExtendedDocument);
        var ptr0 = transition.__destroy_into_raw();
        wasm.documenttransitions_addTransitionReplace(this.ptr, ptr0);
    }
    /**
    * @param {ExtendedDocument} transition
    */
    addTransitionDelete(transition) {
        _assertClass(transition, ExtendedDocument);
        var ptr0 = transition.__destroy_into_raw();
        wasm.documenttransitions_addTransitionDelete(this.ptr, ptr0);
    }
}
/**
*/
export class DocumentValidator {

    static __wrap(ptr) {
        const obj = Object.create(DocumentValidator.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_documentvalidator_free(ptr);
    }
    /**
    * @param {ProtocolVersionValidator} protocol_validator
    */
    constructor(protocol_validator) {
        _assertClass(protocol_validator, ProtocolVersionValidator);
        var ptr0 = protocol_validator.__destroy_into_raw();
        const ret = wasm.documentvalidator_new(ptr0);
        return DocumentValidator.__wrap(ret);
    }
    /**
    * @param {any} js_raw_document
    * @param {DataContract} js_data_contract
    * @returns {ValidationResult}
    */
    validate(js_raw_document, js_data_contract) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(js_data_contract, DataContract);
            wasm.documentvalidator_validate(retptr, this.ptr, addBorrowedObject(js_raw_document), js_data_contract.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return ValidationResult.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
        }
    }
}
/**
*/
export class DocumentsBatchTransition {

    static __wrap(ptr) {
        const obj = Object.create(DocumentsBatchTransition.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_documentsbatchtransition_free(ptr);
    }
    /**
    * @param {any} js_raw_transition
    * @param {Array<any>} data_contracts
    */
    constructor(js_raw_transition, data_contracts) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentsbatchtransition_from_raw_object(retptr, addHeapObject(js_raw_transition), addHeapObject(data_contracts));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return DocumentsBatchTransition.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {number}
    */
    getType() {
        const ret = wasm.documentsbatchtransition_getType(this.ptr);
        return ret;
    }
    /**
    * @returns {any}
    */
    getOwnerId() {
        const ret = wasm.documentsbatchtransition_getOwnerId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {Array<any>}
    */
    getTransitions() {
        const ret = wasm.documentsbatchtransition_getTransitions(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {Array<any>} js_transitions
    */
    setTransitions(js_transitions) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentsbatchtransition_setTransitions(retptr, this.ptr, addHeapObject(js_transitions));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    toJSON() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentsbatchtransition_toJSON(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} js_options
    * @returns {any}
    */
    toObject(js_options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentsbatchtransition_toObject(retptr, this.ptr, addBorrowedObject(js_options));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
        }
    }
    /**
    * @returns {Array<any>}
    */
    getModifiedDataIds() {
        const ret = wasm.documentsbatchtransition_getModifiedDataIds(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number | undefined}
    */
    getSignaturePublicKeyId() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentsbatchtransition_getSignaturePublicKeyId(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r2 = getFloat64Memory0()[retptr / 8 + 1];
            return r0 === 0 ? undefined : r2;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {IdentityPublicKey} identity_public_key
    * @param {Uint8Array} private_key
    * @param {any} bls
    */
    sign(identity_public_key, private_key, bls) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(identity_public_key, IdentityPublicKey);
            const ptr0 = passArray8ToWasm0(private_key, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.documentsbatchtransition_sign(retptr, this.ptr, identity_public_key.ptr, ptr0, len0, addHeapObject(bls));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {IdentityPublicKey} public_key
    */
    verifyPublicKeyLevelAndPurpose(public_key) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(public_key, IdentityPublicKey);
            wasm.documentsbatchtransition_verifyPublicKeyLevelAndPurpose(retptr, this.ptr, public_key.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {IdentityPublicKey} public_key
    */
    verifyPublicKeyIsEnabled(public_key) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(public_key, IdentityPublicKey);
            wasm.documentsbatchtransition_verifyPublicKeyIsEnabled(retptr, this.ptr, public_key.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {IdentityPublicKey} identity_public_key
    * @param {any} bls
    * @returns {boolean}
    */
    verifySignature(identity_public_key, bls) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(identity_public_key, IdentityPublicKey);
            wasm.documentsbatchtransition_verifySignature(retptr, this.ptr, identity_public_key.ptr, addHeapObject(bls));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return r0 !== 0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {number} key_id
    */
    setSignaturePublicKey(key_id) {
        wasm.documentsbatchtransition_setSignaturePublicKey(this.ptr, key_id);
    }
    /**
    * @returns {number}
    */
    getKeySecurityLevelRequirement() {
        const ret = wasm.documentsbatchtransition_getKeySecurityLevelRequirement(this.ptr);
        return ret;
    }
    /**
    * @returns {number}
    */
    getProtocolVersion() {
        const ret = wasm.documentsbatchtransition_getProtocolVersion(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {Uint8Array}
    */
    getSignature() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentsbatchtransition_getSignature(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayU8FromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 1);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Uint8Array} signature
    */
    setSignature(signature) {
        const ptr0 = passArray8ToWasm0(signature, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.documentsbatchtransition_setSignature(this.ptr, ptr0, len0);
    }
    /**
    * @returns {boolean}
    */
    isDocumentStateTransition() {
        const ret = wasm.documentsbatchtransition_isDocumentStateTransition(this.ptr);
        return ret !== 0;
    }
    /**
    * @returns {boolean}
    */
    isDataContractStateTransition() {
        const ret = wasm.documentsbatchtransition_isDataContractStateTransition(this.ptr);
        return ret !== 0;
    }
    /**
    * @returns {boolean}
    */
    isIdentityStateTransition() {
        const ret = wasm.documentsbatchtransition_isIdentityStateTransition(this.ptr);
        return ret !== 0;
    }
    /**
    * @param {StateTransitionExecutionContext} context
    */
    setExecutionContext(context) {
        _assertClass(context, StateTransitionExecutionContext);
        wasm.documentsbatchtransition_setExecutionContext(this.ptr, context.ptr);
    }
    /**
    * @returns {StateTransitionExecutionContext}
    */
    getExecutionContext() {
        const ret = wasm.documentsbatchtransition_getExecutionContext(this.ptr);
        return StateTransitionExecutionContext.__wrap(ret);
    }
    /**
    * @param {any} options
    * @returns {any}
    */
    toBuffer(options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentsbatchtransition_toBuffer(retptr, this.ptr, addBorrowedObject(options));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
        }
    }
    /**
    * @param {any} options
    * @returns {any}
    */
    hash(options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.documentsbatchtransition_hash(retptr, this.ptr, addHeapObject(options));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DummyFeesResult {

    static __wrap(ptr) {
        const obj = Object.create(DummyFeesResult.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_dummyfeesresult_free(ptr);
    }
    /**
    * @returns {bigint}
    */
    get storageFee() {
        const ret = wasm.dummyfeesresult_storageFee(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {bigint}
    */
    get processingFee() {
        const ret = wasm.dummyfeesresult_processingFee(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {Array<any>}
    */
    get feeRefunds() {
        const ret = wasm.dummyfeesresult_feeRefunds(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {any} number
    */
    set storageFee(number) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.dummyfeesresult_set_storageFee(retptr, this.ptr, addHeapObject(number));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} number
    */
    set processingFee(number) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.dummyfeesresult_set_processingFee(retptr, this.ptr, addHeapObject(number));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Array<any>} js_fee_refunds
    */
    set feeRefunds(js_fee_refunds) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.dummyfeesresult_set_feeRefunds(retptr, this.ptr, addHeapObject(js_fee_refunds));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DuplicateDocumentTransitionsWithIdsError {

    static __wrap(ptr) {
        const obj = Object.create(DuplicateDocumentTransitionsWithIdsError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_duplicatedocumenttransitionswithidserror_free(ptr);
    }
    /**
    * @returns {Array<any>}
    */
    getDocumentTransitionReferences() {
        const ret = wasm.duplicatedocumenttransitionswithidserror_getDocumentTransitionReferences(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.duplicatedocumenttransitionswithidserror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicatedocumenttransitionswithidserror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicatedocumenttransitionswithidserror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DuplicateDocumentTransitionsWithIndicesError {

    static __wrap(ptr) {
        const obj = Object.create(DuplicateDocumentTransitionsWithIndicesError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_duplicatedocumenttransitionswithindiceserror_free(ptr);
    }
    /**
    * @returns {Array<any>}
    */
    getDocumentTransitionReferences() {
        const ret = wasm.duplicatedocumenttransitionswithindiceserror_getDocumentTransitionReferences(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.duplicatedocumenttransitionswithindiceserror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicatedocumenttransitionswithindiceserror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicatedocumenttransitionswithindiceserror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DuplicateIndexError {

    static __wrap(ptr) {
        const obj = Object.create(DuplicateIndexError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_duplicateindexerror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getDocumentType() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicateindexerror_getDocumentType(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getIndexName() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicateindexerror_getIndexName(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.duplicateindexerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicateindexerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicateindexerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DuplicateIndexNameError {

    static __wrap(ptr) {
        const obj = Object.create(DuplicateIndexNameError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_duplicateindexnameerror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getDocumentType() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicateindexnameerror_getDocumentType(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getDuplicateIndexName() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicateindexnameerror_getDuplicateIndexName(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.duplicateindexnameerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicateindexnameerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicateindexnameerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DuplicateUniqueIndexError {

    static __wrap(ptr) {
        const obj = Object.create(DuplicateUniqueIndexError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_duplicateuniqueindexerror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getDocumentId() {
        const ret = wasm.duplicateuniqueindexerror_getDocumentId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {Array<any>}
    */
    getDuplicatingProperties() {
        const ret = wasm.duplicateuniqueindexerror_getDuplicatingProperties(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.duplicateuniqueindexerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicateuniqueindexerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicateuniqueindexerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DuplicatedIdentityPublicKeyError {

    static __wrap(ptr) {
        const obj = Object.create(DuplicatedIdentityPublicKeyError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_duplicatedidentitypublickeyerror_free(ptr);
    }
    /**
    * @returns {Array<any>}
    */
    getDuplicatedPublicKeysIds() {
        const ret = wasm.duplicatedidentitypublickeyerror_getDuplicatedPublicKeysIds(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.duplicatedidentitypublickeyerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicatedidentitypublickeyerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicatedidentitypublickeyerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DuplicatedIdentityPublicKeyIdError {

    static __wrap(ptr) {
        const obj = Object.create(DuplicatedIdentityPublicKeyIdError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_duplicatedidentitypublickeyiderror_free(ptr);
    }
    /**
    * @returns {Array<any>}
    */
    getDuplicatedIds() {
        const ret = wasm.duplicatedidentitypublickeyiderror_getDuplicatedIds(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.duplicatedidentitypublickeyiderror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicatedidentitypublickeyiderror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicatedidentitypublickeyiderror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DuplicatedIdentityPublicKeyIdStateError {

    static __wrap(ptr) {
        const obj = Object.create(DuplicatedIdentityPublicKeyIdStateError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_duplicatedidentitypublickeyidstateerror_free(ptr);
    }
    /**
    * @returns {Array<any>}
    */
    getDuplicatedIds() {
        const ret = wasm.duplicatedidentitypublickeyidstateerror_getDuplicatedIds(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.duplicatedidentitypublickeyidstateerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicatedidentitypublickeyidstateerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicatedidentitypublickeyidstateerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class DuplicatedIdentityPublicKeyStateError {

    static __wrap(ptr) {
        const obj = Object.create(DuplicatedIdentityPublicKeyStateError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_duplicatedidentitypublickeystateerror_free(ptr);
    }
    /**
    * @returns {Array<any>}
    */
    getDuplicatedPublicKeysIds() {
        const ret = wasm.duplicatedidentitypublickeystateerror_getDuplicatedPublicKeysIds(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.duplicatedidentitypublickeystateerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicatedidentitypublickeystateerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.duplicatedidentitypublickeystateerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class ExtendedDocument {

    static __wrap(ptr) {
        const obj = Object.create(ExtendedDocument.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_extendeddocument_free(ptr);
    }
    /**
    * @param {any} js_raw_document
    * @param {DataContract} js_data_contract
    */
    constructor(js_raw_document, js_data_contract) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(js_data_contract, DataContract);
            wasm.extendeddocument_new(retptr, addHeapObject(js_raw_document), js_data_contract.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return ExtendedDocument.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {number}
    */
    getProtocolVersion() {
        const ret = wasm.extendeddocument_getProtocolVersion(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {any}
    */
    getId() {
        const ret = wasm.extendeddocument_getId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {any} js_id
    */
    setId(js_id) {
        wasm.extendeddocument_setId(this.ptr, addHeapObject(js_id));
    }
    /**
    * @returns {string}
    */
    getType() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.extendeddocument_getType(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    getDataContractId() {
        const ret = wasm.extendeddocument_getDataContractId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {DataContract}
    */
    getDataContract() {
        const ret = wasm.extendeddocument_getDataContract(this.ptr);
        return DataContract.__wrap(ret);
    }
    /**
    * @param {any} js_data_contract_id
    */
    setDataContractId(js_data_contract_id) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.extendeddocument_setDataContractId(retptr, this.ptr, addBorrowedObject(js_data_contract_id));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
        }
    }
    /**
    * @returns {Document}
    */
    getDocument() {
        const ret = wasm.extendeddocument_getDocument(this.ptr);
        return Document.__wrap(ret);
    }
    /**
    * @param {any} owner_id
    */
    setOwnerId(owner_id) {
        wasm.extendeddocument_setOwnerId(this.ptr, addHeapObject(owner_id));
    }
    /**
    * @returns {any}
    */
    getOwnerId() {
        const ret = wasm.extendeddocument_getOwnerId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {number | undefined} rev
    */
    setRevision(rev) {
        wasm.extendeddocument_setRevision(this.ptr, !isLikeNone(rev), isLikeNone(rev) ? 0 : rev);
    }
    /**
    * @returns {number | undefined}
    */
    getRevision() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.extendeddocument_getRevision(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return r0 === 0 ? undefined : r1 >>> 0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Uint8Array} e
    */
    setEntropy(e) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArray8ToWasm0(e, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.extendeddocument_setEntropy(retptr, this.ptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getEntropy() {
        const ret = wasm.extendeddocument_getEntropy(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {any} d
    */
    setData(d) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.extendeddocument_setData(retptr, this.ptr, addHeapObject(d));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getData() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.extendeddocument_getData(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {string} path
    * @param {any} js_value_to_set
    */
    set(path, js_value_to_set) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passStringToWasm0(path, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.extendeddocument_set(retptr, this.ptr, ptr0, len0, addHeapObject(js_value_to_set));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {string} path
    * @returns {any}
    */
    get(path) {
        const ptr0 = passStringToWasm0(path, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.extendeddocument_get(this.ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
    * @param {Date | undefined} ts
    */
    setCreatedAt(ts) {
        wasm.extendeddocument_setCreatedAt(this.ptr, isLikeNone(ts) ? 0 : addHeapObject(ts));
    }
    /**
    * @param {Date | undefined} ts
    */
    setUpdatedAt(ts) {
        wasm.extendeddocument_setUpdatedAt(this.ptr, isLikeNone(ts) ? 0 : addHeapObject(ts));
    }
    /**
    * @returns {Date | undefined}
    */
    getCreatedAt() {
        const ret = wasm.extendeddocument_getCreatedAt(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {Date | undefined}
    */
    getUpdatedAt() {
        const ret = wasm.extendeddocument_getUpdatedAt(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {Metadata | undefined}
    */
    getMetadata() {
        const ret = wasm.extendeddocument_getMetadata(this.ptr);
        return ret === 0 ? undefined : Metadata.__wrap(ret);
    }
    /**
    * @param {any} metadata
    */
    setMetadata(metadata) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.extendeddocument_setMetadata(retptr, this.ptr, addHeapObject(metadata));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} options
    * @returns {any}
    */
    toObject(options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.extendeddocument_toObject(retptr, this.ptr, addBorrowedObject(options));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
        }
    }
    /**
    * @returns {any}
    */
    toJSON() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.extendeddocument_toJSON(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    toBuffer() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.extendeddocument_toBuffer(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    hash() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.extendeddocument_hash(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {ExtendedDocument}
    */
    clone() {
        const ret = wasm.extendeddocument_clone(this.ptr);
        return ExtendedDocument.__wrap(ret);
    }
}
/**
*/
export class FeeResult {

    static __wrap(ptr) {
        const obj = Object.create(FeeResult.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_feeresult_free(ptr);
    }
    /**
    */
    constructor() {
        const ret = wasm.feeresult_new();
        return FeeResult.__wrap(ret);
    }
    /**
    * @returns {bigint}
    */
    get storageFee() {
        const ret = wasm.feeresult_storageFee(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {bigint}
    */
    get processingFee() {
        const ret = wasm.feeresult_processingFee(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {Array<any>}
    */
    get feeRefunds() {
        const ret = wasm.feeresult_feeRefunds(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {bigint}
    */
    get totalRefunds() {
        const ret = wasm.feeresult_totalRefunds(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {bigint}
    */
    get desiredAmount() {
        const ret = wasm.feeresult_desiredAmount(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {bigint}
    */
    get requiredAmount() {
        const ret = wasm.feeresult_requiredAmount(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {any} number
    */
    set storageFee(number) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.feeresult_set_storageFee(retptr, this.ptr, addHeapObject(number));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} number
    */
    set processingFee(number) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.feeresult_set_processingFee(retptr, this.ptr, addHeapObject(number));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Array<any>} js_fee_refunds
    */
    set feeRefunds(js_fee_refunds) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.feeresult_set_feeRefunds(retptr, this.ptr, addHeapObject(js_fee_refunds));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} number
    */
    set desiredAmount(number) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.feeresult_set_desiredAmount(retptr, this.ptr, addHeapObject(number));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} number
    */
    set requiredAmount(number) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.feeresult_set_requiredAmount(retptr, this.ptr, addHeapObject(number));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} number
    */
    set totalRefunds(number) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.feeresult_set_totalRefunds(retptr, this.ptr, addHeapObject(number));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class FetchAndValidateDataContractFactory {

    static __wrap(ptr) {
        const obj = Object.create(FetchAndValidateDataContractFactory.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_fetchandvalidatedatacontractfactory_free(ptr);
    }
    /**
    * @param {any} state_repository
    */
    constructor(state_repository) {
        const ret = wasm.fetchandvalidatedatacontractfactory_new(addHeapObject(state_repository));
        return FetchAndValidateDataContractFactory.__wrap(ret);
    }
    /**
    * @param {any} js_raw_document
    * @returns {Promise<ValidationResult>}
    */
    validate(js_raw_document) {
        const ret = wasm.fetchandvalidatedatacontractfactory_validate(this.ptr, addHeapObject(js_raw_document));
        return takeObject(ret);
    }
}
/**
*/
export class Identity {

    static __wrap(ptr) {
        const obj = Object.create(Identity.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identity_free(ptr);
    }
    /**
    * @param {any} raw_identity
    */
    constructor(raw_identity) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identity_new(retptr, addHeapObject(raw_identity));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return Identity.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {number}
    */
    getProtocolVersion() {
        const ret = wasm.identity_getProtocolVersion(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {any}
    */
    getId() {
        const ret = wasm.identity_getId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {Array<any>} public_keys
    * @returns {number}
    */
    setPublicKeys(public_keys) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identity_setPublicKeys(retptr, this.ptr, addHeapObject(public_keys));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return r0 >>> 0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any[]}
    */
    getPublicKeys() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identity_getPublicKeys(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 4);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {number} key_id
    * @returns {IdentityPublicKey | undefined}
    */
    getPublicKeyById(key_id) {
        const ret = wasm.identity_getPublicKeyById(this.ptr, key_id);
        return ret === 0 ? undefined : IdentityPublicKey.__wrap(ret);
    }
    /**
    * @returns {number}
    */
    getBalance() {
        const ret = wasm.identity_getBalance(this.ptr);
        return ret;
    }
    /**
    * @param {number} balance
    */
    setBalance(balance) {
        wasm.identity_setBalance(this.ptr, balance);
    }
    /**
    * @param {number} amount
    * @returns {number}
    */
    increaseBalance(amount) {
        const ret = wasm.identity_increaseBalance(this.ptr, amount);
        return ret;
    }
    /**
    * @param {number} amount
    * @returns {number}
    */
    reduceBalance(amount) {
        const ret = wasm.identity_reduceBalance(this.ptr, amount);
        return ret;
    }
    /**
    * @param {any} lock
    */
    setAssetLockProof(lock) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identity_setAssetLockProof(retptr, this.ptr, addHeapObject(lock));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {AssetLockProof | undefined}
    */
    getAssetLockProof() {
        const ret = wasm.identity_getAssetLockProof(this.ptr);
        return ret === 0 ? undefined : AssetLockProof.__wrap(ret);
    }
    /**
    * @param {number} revision
    */
    setRevision(revision) {
        wasm.identity_setRevision(this.ptr, revision);
    }
    /**
    * @returns {number}
    */
    getRevision() {
        const ret = wasm.identity_getRevision(this.ptr);
        return ret;
    }
    /**
    * @param {any} metadata
    */
    setMetadata(metadata) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identity_setMetadata(retptr, this.ptr, addHeapObject(metadata));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {Metadata | undefined}
    */
    getMetadata() {
        const ret = wasm.identity_getMetadata(this.ptr);
        return ret === 0 ? undefined : Metadata.__wrap(ret);
    }
    /**
    * @param {any} object
    * @returns {Identity}
    */
    static from(object) {
        const ret = wasm.identity_from(addHeapObject(object));
        return Identity.__wrap(ret);
    }
    /**
    * @returns {any}
    */
    toJSON() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identity_toJSON(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    toObject() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identity_toObject(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {Uint8Array}
    */
    toBuffer() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identity_toBuffer(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayU8FromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 1);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {Uint8Array}
    */
    hash() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identity_hash(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            var r3 = getInt32Memory0()[retptr / 4 + 3];
            if (r3) {
                throw takeObject(r2);
            }
            var v0 = getArrayU8FromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 1);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {IdentityPublicKey} public_key
    */
    addPublicKey(public_key) {
        _assertClass(public_key, IdentityPublicKey);
        var ptr0 = public_key.__destroy_into_raw();
        wasm.identity_addPublicKey(this.ptr, ptr0);
    }
    /**
    * @param {...any} js_public_keys
    */
    addPublicKeys(...js_public_keys) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identity_addPublicKeys(retptr, this.ptr, addHeapObject(js_public_keys));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {number}
    */
    getPublicKeyMaxId() {
        const ret = wasm.identity_getPublicKeyMaxId(this.ptr);
        return ret;
    }
}
/**
*/
export class IdentityAlreadyExistsError {

    static __wrap(ptr) {
        const obj = Object.create(IdentityAlreadyExistsError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identityalreadyexistserror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getIdentityId() {
        const ret = wasm.identityalreadyexistserror_getIdentityId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.identityalreadyexistserror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityalreadyexistserror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityalreadyexistserror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class IdentityAssetLockProofLockedTransactionMismatchError {

    static __wrap(ptr) {
        const obj = Object.create(IdentityAssetLockProofLockedTransactionMismatchError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identityassetlockprooflockedtransactionmismatcherror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getInstantLockTransactionId() {
        const ret = wasm.identityassetlockprooflockedtransactionmismatcherror_getInstantLockTransactionId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getAssetLockTransactionId() {
        const ret = wasm.identityassetlockprooflockedtransactionmismatcherror_getAssetLockTransactionId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.identityassetlockprooflockedtransactionmismatcherror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityassetlockprooflockedtransactionmismatcherror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityassetlockprooflockedtransactionmismatcherror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class IdentityAssetLockTransactionIsNotFoundError {

    static __wrap(ptr) {
        const obj = Object.create(IdentityAssetLockTransactionIsNotFoundError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identityassetlocktransactionisnotfounderror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getTransactionId() {
        const ret = wasm.identityassetlocktransactionisnotfounderror_getTransactionId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.identityassetlocktransactionisnotfounderror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityassetlocktransactionisnotfounderror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityassetlocktransactionisnotfounderror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class IdentityAssetLockTransactionOutPointAlreadyExistsError {

    static __wrap(ptr) {
        const obj = Object.create(IdentityAssetLockTransactionOutPointAlreadyExistsError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identityassetlocktransactionoutpointalreadyexistserror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getOutputIndex() {
        const ret = wasm.identityassetlocktransactionoutpointalreadyexistserror_getOutputIndex(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {any}
    */
    getTransactionId() {
        const ret = wasm.identityassetlocktransactionoutpointalreadyexistserror_getTransactionId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.identityassetlocktransactionoutpointalreadyexistserror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityassetlocktransactionoutpointalreadyexistserror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityassetlocktransactionoutpointalreadyexistserror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class IdentityAssetLockTransactionOutputNotFoundError {

    static __wrap(ptr) {
        const obj = Object.create(IdentityAssetLockTransactionOutputNotFoundError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identityassetlocktransactionoutputnotfounderror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getOutputIndex() {
        const ret = wasm.identityassetlocktransactionoutputnotfounderror_getOutputIndex(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.identityassetlocktransactionoutputnotfounderror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityassetlocktransactionoutputnotfounderror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityassetlocktransactionoutputnotfounderror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class IdentityCreateTransition {

    static __wrap(ptr) {
        const obj = Object.create(IdentityCreateTransition.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identitycreatetransition_free(ptr);
    }
    /**
    * @param {any} raw_parameters
    */
    constructor(raw_parameters) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitycreatetransition_new(retptr, addHeapObject(raw_parameters));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return IdentityCreateTransition.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} asset_lock_proof
    */
    setAssetLockProof(asset_lock_proof) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitycreatetransition_setAssetLockProof(retptr, this.ptr, addHeapObject(asset_lock_proof));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    get assetLockProof() {
        const ret = wasm.identitycreatetransition_assetLockProof(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getAssetLockProof() {
        const ret = wasm.identitycreatetransition_getAssetLockProof(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {any[]} public_keys
    */
    setPublicKeys(public_keys) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArrayJsValueToWasm0(public_keys, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.identitycreatetransition_setPublicKeys(retptr, this.ptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any[]} public_keys
    */
    addPublicKeys(public_keys) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArrayJsValueToWasm0(public_keys, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.identitycreatetransition_addPublicKeys(retptr, this.ptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any[]}
    */
    getPublicKeys() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitycreatetransition_getPublicKeys(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 4);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any[]}
    */
    get publicKeys() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitycreatetransition_publicKeys(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 4);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {number}
    */
    getType() {
        const ret = wasm.identitycreatetransition_getType(this.ptr);
        return ret;
    }
    /**
    * @returns {any}
    */
    get identityId() {
        const ret = wasm.identitycreatetransition_identityId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getIdentityId() {
        const ret = wasm.identitycreatetransition_getIdentityId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getOwnerId() {
        const ret = wasm.identitycreatetransition_getOwnerId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {any} options
    * @returns {any}
    */
    toObject(options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitycreatetransition_toObject(retptr, this.ptr, addHeapObject(options));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} options
    * @returns {any}
    */
    toBuffer(options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitycreatetransition_toBuffer(retptr, this.ptr, addHeapObject(options));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    toJSON() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitycreatetransition_toJSON(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any[]}
    */
    getModifiedDataIds() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitycreatetransition_getModifiedDataIds(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 4);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {boolean}
    */
    isDataContractStateTransition() {
        const ret = wasm.identitycreatetransition_isDataContractStateTransition(this.ptr);
        return ret !== 0;
    }
    /**
    * @returns {boolean}
    */
    isDocumentStateTransition() {
        const ret = wasm.identitycreatetransition_isDocumentStateTransition(this.ptr);
        return ret !== 0;
    }
    /**
    * @returns {boolean}
    */
    isIdentityStateTransition() {
        const ret = wasm.identitycreatetransition_isIdentityStateTransition(this.ptr);
        return ret !== 0;
    }
    /**
    * @param {StateTransitionExecutionContext} context
    */
    setExecutionContext(context) {
        _assertClass(context, StateTransitionExecutionContext);
        wasm.identitycreatetransition_setExecutionContext(this.ptr, context.ptr);
    }
    /**
    * @returns {StateTransitionExecutionContext}
    */
    getExecutionContext() {
        const ret = wasm.identitycreatetransition_getExecutionContext(this.ptr);
        return StateTransitionExecutionContext.__wrap(ret);
    }
    /**
    * @param {Uint8Array} private_key
    * @param {number} key_type
    * @param {any} bls
    */
    signByPrivateKey(private_key, key_type, bls) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArray8ToWasm0(private_key, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.identitycreatetransition_signByPrivateKey(retptr, this.ptr, ptr0, len0, key_type, addHeapObject(bls));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getSignature() {
        const ret = wasm.identitycreatetransition_getSignature(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {Uint8Array | undefined} signature
    */
    setSignature(signature) {
        var ptr0 = isLikeNone(signature) ? 0 : passArray8ToWasm0(signature, wasm.__wbindgen_malloc);
        var len0 = WASM_VECTOR_LEN;
        wasm.identitycreatetransition_setSignature(this.ptr, ptr0, len0);
    }
}
/**
*/
export class IdentityCreateTransitionBasicValidator {

    static __wrap(ptr) {
        const obj = Object.create(IdentityCreateTransitionBasicValidator.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identitycreatetransitionbasicvalidator_free(ptr);
    }
    /**
    * @param {any} state_repository
    * @param {any} js_bls
    */
    constructor(state_repository, js_bls) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitycreatetransitionbasicvalidator_new(retptr, addHeapObject(state_repository), addHeapObject(js_bls));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return IdentityCreateTransitionBasicValidator.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} raw_state_transition
    * @param {StateTransitionExecutionContext} execution_context
    * @returns {Promise<ValidationResult>}
    */
    validate(raw_state_transition, execution_context) {
        const ptr = this.__destroy_into_raw();
        _assertClass(execution_context, StateTransitionExecutionContext);
        const ret = wasm.identitycreatetransitionbasicvalidator_validate(ptr, addHeapObject(raw_state_transition), execution_context.ptr);
        return takeObject(ret);
    }
}
/**
*/
export class IdentityCreateTransitionStateValidator {

    static __wrap(ptr) {
        const obj = Object.create(IdentityCreateTransitionStateValidator.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identitycreatetransitionstatevalidator_free(ptr);
    }
    /**
    * @param {any} state_repository
    */
    constructor(state_repository) {
        const ret = wasm.identitycreatetransitionstatevalidator_new(addHeapObject(state_repository));
        return IdentityCreateTransitionStateValidator.__wrap(ret);
    }
    /**
    * @param {IdentityCreateTransition} state_transition
    * @returns {Promise<ValidationResult>}
    */
    validate(state_transition) {
        _assertClass(state_transition, IdentityCreateTransition);
        const ret = wasm.identitycreatetransitionstatevalidator_validate(this.ptr, state_transition.ptr);
        return takeObject(ret);
    }
}
/**
*/
export class IdentityFacade {

    static __wrap(ptr) {
        const obj = Object.create(IdentityFacade.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identityfacade_free(ptr);
    }
    /**
    * @param {any} asset_lock_proof
    * @param {Array<any>} public_keys
    * @returns {Identity}
    */
    create(asset_lock_proof, public_keys) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityfacade_create(retptr, this.ptr, addHeapObject(asset_lock_proof), addHeapObject(public_keys));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return Identity.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} identity_object
    * @param {any} options
    * @returns {Identity}
    */
    createFromObject(identity_object, options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityfacade_createFromObject(retptr, this.ptr, addHeapObject(identity_object), addHeapObject(options));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return Identity.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Uint8Array} buffer
    * @param {any} options
    * @returns {Identity}
    */
    createFromBuffer(buffer, options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArray8ToWasm0(buffer, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.identityfacade_createFromBuffer(retptr, this.ptr, ptr0, len0, addHeapObject(options));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return Identity.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Identity} identity
    * @returns {ValidationResult}
    */
    validate(identity) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(identity, Identity);
            wasm.identityfacade_validate(retptr, this.ptr, identity.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return ValidationResult.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Uint8Array} instant_lock
    * @param {Uint8Array} asset_lock_transaction
    * @param {number} output_index
    * @returns {InstantAssetLockProof}
    */
    createInstantAssetLockProof(instant_lock, asset_lock_transaction, output_index) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArray8ToWasm0(instant_lock, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passArray8ToWasm0(asset_lock_transaction, wasm.__wbindgen_malloc);
            const len1 = WASM_VECTOR_LEN;
            wasm.identityfacade_createInstantAssetLockProof(retptr, this.ptr, ptr0, len0, ptr1, len1, output_index);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return InstantAssetLockProof.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {number} core_chain_locked_height
    * @param {Uint8Array} out_point
    * @returns {ChainAssetLockProof}
    */
    createChainAssetLockProof(core_chain_locked_height, out_point) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArray8ToWasm0(out_point, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.identityfacade_createChainAssetLockProof(retptr, this.ptr, core_chain_locked_height, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return ChainAssetLockProof.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Identity} identity
    * @returns {IdentityCreateTransition}
    */
    createIdentityCreateTransition(identity) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(identity, Identity);
            wasm.identityfacade_createIdentityCreateTransition(retptr, this.ptr, identity.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return IdentityCreateTransition.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} identity_id
    * @param {any} asset_lock_proof
    * @returns {IdentityTopUpTransition}
    */
    createIdentityTopUpTransition(identity_id, asset_lock_proof) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityfacade_createIdentityTopUpTransition(retptr, this.ptr, addBorrowedObject(identity_id), addBorrowedObject(asset_lock_proof));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return IdentityTopUpTransition.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
            heap[stack_pointer++] = undefined;
        }
    }
    /**
    * @param {Identity} identity
    * @param {any} public_keys
    * @returns {IdentityUpdateTransition}
    */
    createIdentityUpdateTransition(identity, public_keys) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(identity, Identity);
            wasm.identityfacade_createIdentityUpdateTransition(retptr, this.ptr, identity.ptr, addBorrowedObject(public_keys));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return IdentityUpdateTransition.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
        }
    }
}
/**
*/
export class IdentityFactory {

    static __wrap(ptr) {
        const obj = Object.create(IdentityFactory.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identityfactory_free(ptr);
    }
    /**
    * @param {number} protocol_version
    * @param {IdentityValidator} identity_validator
    */
    constructor(protocol_version, identity_validator) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(identity_validator, IdentityValidator);
            var ptr0 = identity_validator.__destroy_into_raw();
            wasm.identityfactory_new(retptr, protocol_version, ptr0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return IdentityFactory.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} asset_lock_proof
    * @param {Array<any>} public_keys
    * @returns {Identity}
    */
    create(asset_lock_proof, public_keys) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityfactory_create(retptr, this.ptr, addHeapObject(asset_lock_proof), addHeapObject(public_keys));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return Identity.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} identity_object
    * @param {any} options
    * @returns {Identity}
    */
    createFromObject(identity_object, options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityfactory_createFromObject(retptr, this.ptr, addHeapObject(identity_object), addHeapObject(options));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return Identity.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Uint8Array} buffer
    * @param {any} options
    * @returns {Identity}
    */
    createFromBuffer(buffer, options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArray8ToWasm0(buffer, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.identityfactory_createFromBuffer(retptr, this.ptr, ptr0, len0, addHeapObject(options));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return Identity.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Uint8Array} instant_lock
    * @param {Uint8Array} asset_lock_transaction
    * @param {number} output_index
    * @returns {InstantAssetLockProof}
    */
    createInstantAssetLockProof(instant_lock, asset_lock_transaction, output_index) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArray8ToWasm0(instant_lock, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passArray8ToWasm0(asset_lock_transaction, wasm.__wbindgen_malloc);
            const len1 = WASM_VECTOR_LEN;
            wasm.identityfactory_createInstantAssetLockProof(retptr, this.ptr, ptr0, len0, ptr1, len1, output_index);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return InstantAssetLockProof.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {number} core_chain_locked_height
    * @param {Uint8Array} out_point
    * @returns {ChainAssetLockProof}
    */
    createChainAssetLockProof(core_chain_locked_height, out_point) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArray8ToWasm0(out_point, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.identityfactory_createChainAssetLockProof(retptr, this.ptr, core_chain_locked_height, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return ChainAssetLockProof.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Identity} identity
    * @returns {IdentityCreateTransition}
    */
    createIdentityCreateTransition(identity) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(identity, Identity);
            wasm.identityfactory_createIdentityCreateTransition(retptr, this.ptr, identity.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return IdentityCreateTransition.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} identity_id
    * @param {any} asset_lock_proof
    * @returns {IdentityTopUpTransition}
    */
    createIdentityTopUpTransition(identity_id, asset_lock_proof) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityfactory_createIdentityTopUpTransition(retptr, this.ptr, addBorrowedObject(identity_id), addBorrowedObject(asset_lock_proof));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return IdentityTopUpTransition.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
            heap[stack_pointer++] = undefined;
        }
    }
    /**
    * @param {Identity} identity
    * @param {any} public_keys
    * @returns {IdentityUpdateTransition}
    */
    createIdentityUpdateTransition(identity, public_keys) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(identity, Identity);
            wasm.identityfactory_createIdentityUpdateTransition(retptr, this.ptr, identity.ptr, addBorrowedObject(public_keys));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return IdentityUpdateTransition.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
        }
    }
}
/**
*/
export class IdentityInsufficientBalanceError {

    static __wrap(ptr) {
        const obj = Object.create(IdentityInsufficientBalanceError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identityinsufficientbalanceerror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getIdentityId() {
        const ret = wasm.identityinsufficientbalanceerror_getIdentityId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getBalance() {
        const ret = wasm.identityinsufficientbalanceerror_getBalance(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.identityinsufficientbalanceerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityinsufficientbalanceerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityinsufficientbalanceerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class IdentityNotFoundError {

    static __wrap(ptr) {
        const obj = Object.create(IdentityNotFoundError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identitynotfounderror_free(ptr);
    }
    /**
    * @param {any} identity_id
    */
    constructor(identity_id) {
        const ret = wasm.identitynotfounderror_new(addHeapObject(identity_id));
        return IdentityNotFoundError.__wrap(ret);
    }
    /**
    * @returns {any}
    */
    getIdentityId() {
        const ret = wasm.identitynotfounderror_getIdentityId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.identitynotfounderror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitynotfounderror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitynotfounderror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class IdentityPublicKey {

    static __wrap(ptr) {
        const obj = Object.create(IdentityPublicKey.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identitypublickey_free(ptr);
    }
    /**
    * @param {any} raw_public_key
    */
    constructor(raw_public_key) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitypublickey_new(retptr, addHeapObject(raw_public_key));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return IdentityPublicKey.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {number}
    */
    getId() {
        const ret = wasm.identitypublickey_getId(this.ptr);
        return ret >>> 0;
    }
    /**
    * @param {number} id
    */
    setId(id) {
        wasm.identitypublickey_setId(this.ptr, id);
    }
    /**
    * @returns {number}
    */
    getType() {
        const ret = wasm.identitypublickey_getType(this.ptr);
        return ret;
    }
    /**
    * @param {number} key_type
    */
    setType(key_type) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitypublickey_setType(retptr, this.ptr, key_type);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Uint8Array} data
    */
    setData(data) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.identitypublickey_setData(retptr, this.ptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getData() {
        const ret = wasm.identitypublickey_getData(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {number} purpose
    */
    setPurpose(purpose) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitypublickey_setPurpose(retptr, this.ptr, purpose);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {number}
    */
    getPurpose() {
        const ret = wasm.identitypublickey_getPurpose(this.ptr);
        return ret;
    }
    /**
    * @param {number} security_level
    */
    setSecurityLevel(security_level) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitypublickey_setSecurityLevel(retptr, this.ptr, security_level);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {number}
    */
    getSecurityLevel() {
        const ret = wasm.identitypublickey_getSecurityLevel(this.ptr);
        return ret;
    }
    /**
    * @param {boolean} read_only
    */
    setReadOnly(read_only) {
        wasm.identitypublickey_setReadOnly(this.ptr, read_only);
    }
    /**
    * @returns {boolean}
    */
    isReadOnly() {
        const ret = wasm.identitypublickey_isReadOnly(this.ptr);
        return ret !== 0;
    }
    /**
    * @param {Date} timestamp
    */
    setDisabledAt(timestamp) {
        wasm.identitypublickey_setDisabledAt(this.ptr, addHeapObject(timestamp));
    }
    /**
    * @returns {Date | undefined}
    */
    getDisabledAt() {
        const ret = wasm.identitypublickey_getDisabledAt(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {Uint8Array}
    */
    hash() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitypublickey_hash(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            var r3 = getInt32Memory0()[retptr / 4 + 3];
            if (r3) {
                throw takeObject(r2);
            }
            var v0 = getArrayU8FromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 1);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {boolean}
    */
    isMaster() {
        const ret = wasm.identitypublickey_isMaster(this.ptr);
        return ret !== 0;
    }
    /**
    * @returns {any}
    */
    toJSON() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitypublickey_toJSON(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    toObject() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitypublickey_toObject(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class IdentityPublicKeyDisabledAtWindowViolationError {

    static __wrap(ptr) {
        const obj = Object.create(IdentityPublicKeyDisabledAtWindowViolationError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identitypublickeydisabledatwindowviolationerror_free(ptr);
    }
    /**
    * @returns {Date}
    */
    getDisabledAt() {
        const ret = wasm.identitypublickeydisabledatwindowviolationerror_getDisabledAt(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {Date}
    */
    getTimeWindowStart() {
        const ret = wasm.identitypublickeydisabledatwindowviolationerror_getTimeWindowStart(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {Date}
    */
    getTimeWindowEnd() {
        const ret = wasm.identitypublickeydisabledatwindowviolationerror_getTimeWindowEnd(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.identitypublickeydisabledatwindowviolationerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitypublickeydisabledatwindowviolationerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitypublickeydisabledatwindowviolationerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class IdentityPublicKeyIsDisabledError {

    static __wrap(ptr) {
        const obj = Object.create(IdentityPublicKeyIsDisabledError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identitypublickeyisdisablederror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getPublicKeyIndex() {
        const ret = wasm.identitypublickeyisdisablederror_getPublicKeyIndex(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.identitypublickeyisdisablederror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitypublickeyisdisablederror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitypublickeyisdisablederror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class IdentityPublicKeyIsReadOnlyError {

    static __wrap(ptr) {
        const obj = Object.create(IdentityPublicKeyIsReadOnlyError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identitypublickeyisreadonlyerror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getKeyId() {
        const ret = wasm.identitypublickeyisreadonlyerror_getKeyId(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getPublicKeyIndex() {
        const ret = wasm.identitypublickeyisreadonlyerror_getPublicKeyIndex(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.identitypublickeyisreadonlyerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitypublickeyisreadonlyerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitypublickeyisreadonlyerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class IdentityPublicKeyWithWitness {

    static __wrap(ptr) {
        const obj = Object.create(IdentityPublicKeyWithWitness.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identitypublickeywithwitness_free(ptr);
    }
    /**
    * @param {any} raw_public_key
    */
    constructor(raw_public_key) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitypublickeywithwitness_new(retptr, addHeapObject(raw_public_key));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return IdentityPublicKeyWithWitness.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {number}
    */
    getId() {
        const ret = wasm.identitypublickeywithwitness_getId(this.ptr);
        return ret >>> 0;
    }
    /**
    * @param {number} id
    */
    setId(id) {
        wasm.identitypublickeywithwitness_setId(this.ptr, id);
    }
    /**
    * @returns {number}
    */
    getType() {
        const ret = wasm.identitypublickeywithwitness_getType(this.ptr);
        return ret;
    }
    /**
    * @param {number} key_type
    */
    setType(key_type) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitypublickeywithwitness_setType(retptr, this.ptr, key_type);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Uint8Array} data
    */
    setData(data) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.identitypublickeywithwitness_setData(retptr, this.ptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getData() {
        const ret = wasm.identitypublickeywithwitness_getData(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {number} purpose
    */
    setPurpose(purpose) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitypublickeywithwitness_setPurpose(retptr, this.ptr, purpose);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {number}
    */
    getPurpose() {
        const ret = wasm.identitypublickeywithwitness_getPurpose(this.ptr);
        return ret;
    }
    /**
    * @param {number} security_level
    */
    setSecurityLevel(security_level) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitypublickeywithwitness_setSecurityLevel(retptr, this.ptr, security_level);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {number}
    */
    getSecurityLevel() {
        const ret = wasm.identitypublickeywithwitness_getSecurityLevel(this.ptr);
        return ret;
    }
    /**
    * @param {boolean} read_only
    */
    setReadOnly(read_only) {
        wasm.identitypublickeywithwitness_setReadOnly(this.ptr, read_only);
    }
    /**
    * @returns {boolean}
    */
    isReadOnly() {
        const ret = wasm.identitypublickeywithwitness_isReadOnly(this.ptr);
        return ret !== 0;
    }
    /**
    * @param {Uint8Array} signature
    */
    setSignature(signature) {
        const ptr0 = passArray8ToWasm0(signature, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.identitypublickeywithwitness_setSignature(this.ptr, ptr0, len0);
    }
    /**
    * @returns {Uint8Array}
    */
    getSignature() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitypublickeywithwitness_getSignature(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayU8FromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 1);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {Uint8Array}
    */
    hash() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitypublickeywithwitness_hash(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            var r3 = getInt32Memory0()[retptr / 4 + 3];
            if (r3) {
                throw takeObject(r2);
            }
            var v0 = getArrayU8FromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 1);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {boolean}
    */
    isMaster() {
        const ret = wasm.identitypublickeywithwitness_isMaster(this.ptr);
        return ret !== 0;
    }
    /**
    * @returns {any}
    */
    toJSON() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitypublickeywithwitness_toJSON(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} options
    * @returns {any}
    */
    toObject(options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitypublickeywithwitness_toObject(retptr, this.ptr, addHeapObject(options));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class IdentityTopUpTransition {

    static __wrap(ptr) {
        const obj = Object.create(IdentityTopUpTransition.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identitytopuptransition_free(ptr);
    }
    /**
    * @param {any} raw_parameters
    */
    constructor(raw_parameters) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitytopuptransition_new(retptr, addHeapObject(raw_parameters));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return IdentityTopUpTransition.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} asset_lock_proof
    */
    setAssetLockProof(asset_lock_proof) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitytopuptransition_setAssetLockProof(retptr, this.ptr, addHeapObject(asset_lock_proof));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    get assetLockProof() {
        const ret = wasm.identitytopuptransition_assetLockProof(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getAssetLockProof() {
        const ret = wasm.identitytopuptransition_getAssetLockProof(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getType() {
        const ret = wasm.identitytopuptransition_getType(this.ptr);
        return ret;
    }
    /**
    * @returns {any}
    */
    get identityId() {
        const ret = wasm.identitytopuptransition_identityId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getIdentityId() {
        const ret = wasm.identitytopuptransition_getIdentityId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getOwnerId() {
        const ret = wasm.identitytopuptransition_getOwnerId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {any} options
    * @returns {any}
    */
    toObject(options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitytopuptransition_toObject(retptr, this.ptr, addHeapObject(options));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} options
    * @returns {any}
    */
    toBuffer(options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitytopuptransition_toBuffer(retptr, this.ptr, addHeapObject(options));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    toJSON() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitytopuptransition_toJSON(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any[]}
    */
    getModifiedDataIds() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitytopuptransition_getModifiedDataIds(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 4);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {boolean}
    */
    isDataContractStateTransition() {
        const ret = wasm.identitytopuptransition_isDataContractStateTransition(this.ptr);
        return ret !== 0;
    }
    /**
    * @returns {boolean}
    */
    isDocumentStateTransition() {
        const ret = wasm.identitytopuptransition_isDocumentStateTransition(this.ptr);
        return ret !== 0;
    }
    /**
    * @returns {boolean}
    */
    isIdentityStateTransition() {
        const ret = wasm.identitytopuptransition_isIdentityStateTransition(this.ptr);
        return ret !== 0;
    }
    /**
    * @param {StateTransitionExecutionContext} context
    */
    setExecutionContext(context) {
        _assertClass(context, StateTransitionExecutionContext);
        wasm.identitytopuptransition_setExecutionContext(this.ptr, context.ptr);
    }
    /**
    * @returns {StateTransitionExecutionContext}
    */
    getExecutionContext() {
        const ret = wasm.identitytopuptransition_getExecutionContext(this.ptr);
        return StateTransitionExecutionContext.__wrap(ret);
    }
    /**
    * @param {Uint8Array} private_key
    * @param {number} key_type
    * @param {any} bls
    */
    signByPrivateKey(private_key, key_type, bls) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArray8ToWasm0(private_key, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.identitytopuptransition_signByPrivateKey(retptr, this.ptr, ptr0, len0, key_type, addHeapObject(bls));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getSignature() {
        const ret = wasm.identitytopuptransition_getSignature(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {Uint8Array | undefined} signature
    */
    setSignature(signature) {
        var ptr0 = isLikeNone(signature) ? 0 : passArray8ToWasm0(signature, wasm.__wbindgen_malloc);
        var len0 = WASM_VECTOR_LEN;
        wasm.identitytopuptransition_setSignature(this.ptr, ptr0, len0);
    }
}
/**
*/
export class IdentityTopUpTransitionBasicValidator {

    static __wrap(ptr) {
        const obj = Object.create(IdentityTopUpTransitionBasicValidator.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identitytopuptransitionbasicvalidator_free(ptr);
    }
    /**
    * @param {any} state_repository
    */
    constructor(state_repository) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identitytopuptransitionbasicvalidator_new(retptr, addHeapObject(state_repository));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return IdentityTopUpTransitionBasicValidator.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} raw_state_transition
    * @param {StateTransitionExecutionContext} execution_context
    * @returns {Promise<ValidationResult>}
    */
    validate(raw_state_transition, execution_context) {
        const ptr = this.__destroy_into_raw();
        _assertClass(execution_context, StateTransitionExecutionContext);
        const ret = wasm.identitytopuptransitionbasicvalidator_validate(ptr, addHeapObject(raw_state_transition), execution_context.ptr);
        return takeObject(ret);
    }
}
/**
*/
export class IdentityTopUpTransitionStateValidator {

    static __wrap(ptr) {
        const obj = Object.create(IdentityTopUpTransitionStateValidator.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identitytopuptransitionstatevalidator_free(ptr);
    }
    /**
    * @param {any} state_repository
    */
    constructor(state_repository) {
        const ret = wasm.identitytopuptransitionstatevalidator_new(addHeapObject(state_repository));
        return IdentityTopUpTransitionStateValidator.__wrap(ret);
    }
    /**
    * @param {IdentityTopUpTransition} state_transition
    * @returns {Promise<ValidationResult>}
    */
    validate(state_transition) {
        _assertClass(state_transition, IdentityTopUpTransition);
        const ret = wasm.identitytopuptransitionstatevalidator_validate(this.ptr, state_transition.ptr);
        return takeObject(ret);
    }
}
/**
*/
export class IdentityUpdatePublicKeysValidator {

    static __wrap(ptr) {
        const obj = Object.create(IdentityUpdatePublicKeysValidator.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identityupdatepublickeysvalidator_free(ptr);
    }
    /**
    */
    constructor() {
        const ret = wasm.identityupdatepublickeysvalidator_new();
        return IdentityUpdatePublicKeysValidator.__wrap(ret);
    }
    /**
    * @param {any[]} raw_public_keys
    * @returns {ValidationResult}
    */
    validate(raw_public_keys) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArrayJsValueToWasm0(raw_public_keys, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.identityupdatepublickeysvalidator_validate(retptr, this.ptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return ValidationResult.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class IdentityUpdateTransition {

    static __wrap(ptr) {
        const obj = Object.create(IdentityUpdateTransition.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identityupdatetransition_free(ptr);
    }
    /**
    * @param {any} raw_parameters
    */
    constructor(raw_parameters) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityupdatetransition_new(retptr, addHeapObject(raw_parameters));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return IdentityUpdateTransition.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any[] | undefined} public_keys
    */
    setPublicKeysToAdd(public_keys) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            var ptr0 = isLikeNone(public_keys) ? 0 : passArrayJsValueToWasm0(public_keys, wasm.__wbindgen_malloc);
            var len0 = WASM_VECTOR_LEN;
            wasm.identityupdatetransition_setPublicKeysToAdd(retptr, this.ptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any[]}
    */
    getPublicKeysToAdd() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityupdatetransition_getPublicKeysToAdd(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 4);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any[]}
    */
    get addPublicKeys() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityupdatetransition_addPublicKeys(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 4);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any[]}
    */
    getPublicKeyIdsToDisable() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityupdatetransition_getPublicKeyIdsToDisable(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 4);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Uint32Array | undefined} public_key_ids
    */
    setPublicKeyIdsToDisable(public_key_ids) {
        var ptr0 = isLikeNone(public_key_ids) ? 0 : passArray32ToWasm0(public_key_ids, wasm.__wbindgen_malloc);
        var len0 = WASM_VECTOR_LEN;
        wasm.identityupdatetransition_setPublicKeyIdsToDisable(this.ptr, ptr0, len0);
    }
    /**
    * @returns {Date | undefined}
    */
    getPublicKeysDisabledAt() {
        const ret = wasm.identityupdatetransition_getPublicKeysDisabledAt(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {Date | undefined} timestamp
    */
    setPublicKeysDisabledAt(timestamp) {
        wasm.identityupdatetransition_setPublicKeysDisabledAt(this.ptr, isLikeNone(timestamp) ? 0 : addHeapObject(timestamp));
    }
    /**
    * @returns {number}
    */
    getType() {
        const ret = wasm.identityupdatetransition_getType(this.ptr);
        return ret;
    }
    /**
    * @returns {any}
    */
    get identityId() {
        const ret = wasm.identityupdatetransition_identityId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getIdentityId() {
        const ret = wasm.identityupdatetransition_getIdentityId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {any} identity_id
    */
    setIdentityId(identity_id) {
        try {
            wasm.identityupdatetransition_setIdentityId(this.ptr, addBorrowedObject(identity_id));
        } finally {
            heap[stack_pointer++] = undefined;
        }
    }
    /**
    * @returns {any}
    */
    getOwnerId() {
        const ret = wasm.identityupdatetransition_getOwnerId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {any} options
    * @returns {any}
    */
    toObject(options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityupdatetransition_toObject(retptr, this.ptr, addHeapObject(options));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} options
    * @returns {any}
    */
    toBuffer(options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityupdatetransition_toBuffer(retptr, this.ptr, addHeapObject(options));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    toJSON() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityupdatetransition_toJSON(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any[]}
    */
    getModifiedDataIds() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityupdatetransition_getModifiedDataIds(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 4);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {boolean}
    */
    isDataContractStateTransition() {
        const ret = wasm.identityupdatetransition_isDataContractStateTransition(this.ptr);
        return ret !== 0;
    }
    /**
    * @returns {boolean}
    */
    isDocumentStateTransition() {
        const ret = wasm.identityupdatetransition_isDocumentStateTransition(this.ptr);
        return ret !== 0;
    }
    /**
    * @returns {boolean}
    */
    isIdentityStateTransition() {
        const ret = wasm.identityupdatetransition_isIdentityStateTransition(this.ptr);
        return ret !== 0;
    }
    /**
    * @param {StateTransitionExecutionContext} context
    */
    setExecutionContext(context) {
        _assertClass(context, StateTransitionExecutionContext);
        wasm.identityupdatetransition_setExecutionContext(this.ptr, context.ptr);
    }
    /**
    * @returns {StateTransitionExecutionContext}
    */
    getExecutionContext() {
        const ret = wasm.identityupdatetransition_getExecutionContext(this.ptr);
        return StateTransitionExecutionContext.__wrap(ret);
    }
    /**
    * @param {Uint8Array} private_key
    * @param {number} key_type
    * @param {any} bls
    */
    signByPrivateKey(private_key, key_type, bls) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArray8ToWasm0(private_key, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.identityupdatetransition_signByPrivateKey(retptr, this.ptr, ptr0, len0, key_type, addHeapObject(bls));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {number | undefined} key_id
    */
    setSignaturePublicKeyId(key_id) {
        wasm.identityupdatetransition_setSignaturePublicKeyId(this.ptr, !isLikeNone(key_id), isLikeNone(key_id) ? 0 : key_id);
    }
    /**
    * @returns {any}
    */
    getSignature() {
        const ret = wasm.identityupdatetransition_getSignature(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {Uint8Array | undefined} signature
    */
    setSignature(signature) {
        var ptr0 = isLikeNone(signature) ? 0 : passArray8ToWasm0(signature, wasm.__wbindgen_malloc);
        var len0 = WASM_VECTOR_LEN;
        wasm.identityupdatetransition_setSignature(this.ptr, ptr0, len0);
    }
    /**
    * @returns {number}
    */
    getRevision() {
        const ret = wasm.identityupdatetransition_getRevision(this.ptr);
        return ret >>> 0;
    }
    /**
    * @param {number} revision
    */
    setRevision(revision) {
        wasm.identityupdatetransition_setRevision(this.ptr, revision);
    }
    /**
    * @param {IdentityPublicKey} identity_public_key
    * @param {Uint8Array} private_key
    * @param {any} bls
    */
    sign(identity_public_key, private_key, bls) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(identity_public_key, IdentityPublicKey);
            const ptr0 = passArray8ToWasm0(private_key, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.identityupdatetransition_sign(retptr, this.ptr, identity_public_key.ptr, ptr0, len0, addHeapObject(bls));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {IdentityPublicKey} identity_public_key
    * @param {any} bls
    * @returns {boolean}
    */
    verifySignature(identity_public_key, bls) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            _assertClass(identity_public_key, IdentityPublicKey);
            wasm.identityupdatetransition_verifySignature(retptr, this.ptr, identity_public_key.ptr, addHeapObject(bls));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return r0 !== 0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class IdentityUpdateTransitionBasicValidator {

    static __wrap(ptr) {
        const obj = Object.create(IdentityUpdateTransitionBasicValidator.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identityupdatetransitionbasicvalidator_free(ptr);
    }
    /**
    * @param {any} js_bls
    */
    constructor(js_bls) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityupdatetransitionbasicvalidator_new(retptr, addHeapObject(js_bls));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return IdentityUpdateTransitionBasicValidator.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} raw_state_transition
    * @returns {ValidationResult}
    */
    validate(raw_state_transition) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityupdatetransitionbasicvalidator_validate(retptr, this.ptr, addHeapObject(raw_state_transition));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return ValidationResult.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class IdentityUpdateTransitionStateValidator {

    static __wrap(ptr) {
        const obj = Object.create(IdentityUpdateTransitionStateValidator.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identityupdatetransitionstatevalidator_free(ptr);
    }
    /**
    * @param {any} state_repository
    */
    constructor(state_repository) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityupdatetransitionstatevalidator_new(retptr, addHeapObject(state_repository));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return IdentityUpdateTransitionStateValidator.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {IdentityUpdateTransition} state_transition
    * @returns {Promise<ValidationResult>}
    */
    validate(state_transition) {
        _assertClass(state_transition, IdentityUpdateTransition);
        const ret = wasm.identityupdatetransitionstatevalidator_validate(this.ptr, state_transition.ptr);
        return takeObject(ret);
    }
}
/**
*/
export class IdentityValidator {

    static __wrap(ptr) {
        const obj = Object.create(IdentityValidator.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_identityvalidator_free(ptr);
    }
    /**
    * @param {any} bls
    */
    constructor(bls) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityvalidator_new(retptr, addHeapObject(bls));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return IdentityValidator.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} raw_identity
    * @returns {ValidationResult}
    */
    validate(raw_identity) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.identityvalidator_validate(retptr, this.ptr, addHeapObject(raw_identity));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return ValidationResult.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class IncompatibleDataContractSchemaError {

    static __wrap(ptr) {
        const obj = Object.create(IncompatibleDataContractSchemaError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_incompatibledatacontractschemaerror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getDataContractId() {
        const ret = wasm.incompatibledatacontractschemaerror_getDataContractId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {string}
    */
    getOperation() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.incompatibledatacontractschemaerror_getOperation(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getFieldPath() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.incompatibledatacontractschemaerror_getFieldPath(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.incompatibledatacontractschemaerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.incompatibledatacontractschemaerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.incompatibledatacontractschemaerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class IncompatibleProtocolVersionError {

    static __wrap(ptr) {
        const obj = Object.create(IncompatibleProtocolVersionError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_incompatibleprotocolversionerror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getParsedProtocolVersion() {
        const ret = wasm.incompatibleprotocolversionerror_getParsedProtocolVersion(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getMinimalProtocolVersion() {
        const ret = wasm.incompatibleprotocolversionerror_getMinimalProtocolVersion(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.incompatibleprotocolversionerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.incompatibleprotocolversionerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.incompatibleprotocolversionerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class IncompatibleRe2PatternError {

    static __wrap(ptr) {
        const obj = Object.create(IncompatibleRe2PatternError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_incompatiblere2patternerror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getPattern() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.incompatiblere2patternerror_getPattern(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getPath() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.incompatiblere2patternerror_getPath(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getMessage() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.incompatiblere2patternerror_getMessage(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.incompatiblere2patternerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.incompatiblere2patternerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.incompatiblere2patternerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InconsistentCompoundIndexDataError {

    static __wrap(ptr) {
        const obj = Object.create(InconsistentCompoundIndexDataError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_inconsistentcompoundindexdataerror_free(ptr);
    }
    /**
    * @returns {Array<any>}
    */
    getIndexedProperties() {
        const ret = wasm.inconsistentcompoundindexdataerror_getIndexedProperties(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {string}
    */
    getDocumentType() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.inconsistentcompoundindexdataerror_getDocumentType(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.inconsistentcompoundindexdataerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.inconsistentcompoundindexdataerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.inconsistentcompoundindexdataerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class IndexDefinition {

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_indexdefinition_free(ptr);
    }
    /**
    * @returns {string}
    */
    getName() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.indexdefinition_getName(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any[]}
    */
    getProperties() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.indexdefinition_getProperties(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 4);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {boolean}
    */
    isUnique() {
        const ret = wasm.indexdefinition_isUnique(this.ptr);
        return ret !== 0;
    }
    /**
    * @returns {any}
    */
    toObject() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.indexdefinition_toObject(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class IndexProperty {

    static __wrap(ptr) {
        const obj = Object.create(IndexProperty.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_indexproperty_free(ptr);
    }
    /**
    * @returns {string}
    */
    getName() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.indexproperty_getName(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {boolean}
    */
    isAscending() {
        const ret = wasm.indexproperty_isAscending(this.ptr);
        return ret !== 0;
    }
}
/**
*/
export class InstantAssetLockProof {

    static __wrap(ptr) {
        const obj = Object.create(InstantAssetLockProof.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_instantassetlockproof_free(ptr);
    }
    /**
    * @param {any} raw_parameters
    */
    constructor(raw_parameters) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.instantassetlockproof_new(retptr, addHeapObject(raw_parameters));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return InstantAssetLockProof.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {number}
    */
    getType() {
        const ret = wasm.instantassetlockproof_getType(this.ptr);
        return ret;
    }
    /**
    * @returns {number}
    */
    getOutputIndex() {
        const ret = wasm.instantassetlockproof_getOutputIndex(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {any | undefined}
    */
    getOutPoint() {
        const ret = wasm.instantassetlockproof_getOutPoint(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getOutput() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.instantassetlockproof_getOutput(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    createIdentifier() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.instantassetlockproof_createIdentifier(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getInstantLock() {
        const ret = wasm.instantassetlockproof_getInstantLock(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getTransaction() {
        const ret = wasm.instantassetlockproof_getTransaction(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    toObject() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.instantassetlockproof_toObject(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    toJSON() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.instantassetlockproof_toJSON(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InstantAssetLockProofStructureValidator {

    static __wrap(ptr) {
        const obj = Object.create(InstantAssetLockProofStructureValidator.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_instantassetlockproofstructurevalidator_free(ptr);
    }
    /**
    * @param {any} state_repository
    */
    constructor(state_repository) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.instantassetlockproofstructurevalidator_new(retptr, addHeapObject(state_repository));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return InstantAssetLockProofStructureValidator.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} raw_asset_lock_proof
    * @param {StateTransitionExecutionContext} execution_context
    * @returns {Promise<ValidationResult>}
    */
    validate(raw_asset_lock_proof, execution_context) {
        _assertClass(execution_context, StateTransitionExecutionContext);
        const ret = wasm.instantassetlockproofstructurevalidator_validate(this.ptr, addHeapObject(raw_asset_lock_proof), execution_context.ptr);
        return takeObject(ret);
    }
}
/**
*/
export class InvalidActionError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidActionError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalidactionerror_free(ptr);
    }
}
/**
*/
export class InvalidActionNameError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidActionNameError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalidactionnameerror_free(ptr);
    }
    /**
    * @param {any[]} actions
    */
    constructor(actions) {
        const ptr0 = passArrayJsValueToWasm0(actions, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.invalidactionnameerror_new(ptr0, len0);
        return InvalidActionNameError.__wrap(ret);
    }
    /**
    * @returns {any[]}
    */
    getActions() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidactionnameerror_getActions(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 4);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}

export class InvalidActiontError {

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalidactionterror_free(ptr);
    }
    /**
    * @param {any} action
    */
    constructor(action) {
        const ret = wasm.invalidactionterror_new(addHeapObject(action));
        return InvalidActionError.__wrap(ret);
    }
}
/**
*/
export class InvalidAssetLockProofCoreChainHeightError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidAssetLockProofCoreChainHeightError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalidassetlockproofcorechainheighterror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getProofCoreChainLockedHeight() {
        const ret = wasm.invalidassetlockproofcorechainheighterror_getProofCoreChainLockedHeight(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getCurrentCoreChainLockedHeight() {
        const ret = wasm.invalidassetlockproofcorechainheighterror_getCurrentCoreChainLockedHeight(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalidassetlockproofcorechainheighterror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidassetlockproofcorechainheighterror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidassetlockproofcorechainheighterror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidAssetLockProofTransactionHeightError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidAssetLockProofTransactionHeightError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalidassetlockprooftransactionheighterror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getProofCoreChainLockedHeight() {
        const ret = wasm.invalidassetlockprooftransactionheighterror_getProofCoreChainLockedHeight(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number | undefined}
    */
    getTransactionHeight() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidassetlockprooftransactionheighterror_getTransactionHeight(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return r0 === 0 ? undefined : r1 >>> 0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalidassetlockprooftransactionheighterror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidassetlockprooftransactionheighterror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidassetlockprooftransactionheighterror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidAssetLockTransactionOutputReturnSizeError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidAssetLockTransactionOutputReturnSizeError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalidassetlocktransactionoutputreturnsizeerror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getOutputIndex() {
        const ret = wasm.invalidassetlocktransactionoutputreturnsizeerror_getOutputIndex(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalidassetlocktransactionoutputreturnsizeerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidassetlocktransactionoutputreturnsizeerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidassetlocktransactionoutputreturnsizeerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidCompoundIndexError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidCompoundIndexError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalidcompoundindexerror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getDocumentType() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidcompoundindexerror_getDocumentType(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getIndexName() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidcompoundindexerror_getIndexName(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalidcompoundindexerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidcompoundindexerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidcompoundindexerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidDataContractError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidDataContractError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invaliddatacontracterror_free(ptr);
    }
    /**
    * @returns {any[]}
    */
    getErrors() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invaliddatacontracterror_getErrors(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 4);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getRawDataContract() {
        const ret = wasm.invaliddatacontracterror_getRawDataContract(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {string}
    */
    getMessage() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invaliddatacontracterror_getMessage(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
}
/**
*/
export class InvalidDataContractIdError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidDataContractIdError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invaliddatacontractiderror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getExpectedId() {
        const ret = wasm.invaliddatacontractiderror_getExpectedId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getInvalidId() {
        const ret = wasm.invaliddatacontractiderror_getInvalidId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invaliddatacontractiderror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invaliddatacontractiderror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invaliddatacontractiderror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidDataContractVersionError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidDataContractVersionError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invaliddatacontractversionerror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getExpectedVersion() {
        const ret = wasm.invaliddatacontractversionerror_getExpectedVersion(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getVersion() {
        const ret = wasm.invaliddatacontractversionerror_getVersion(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invaliddatacontractversionerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invaliddatacontractversionerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invaliddatacontractversionerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidDocumentActionError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidDocumentActionError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invaliddocumentactionerror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getDocumentTransition() {
        const ret = wasm.invaliddocumentactionerror_getDocumentTransition(this.ptr);
        return takeObject(ret);
    }
}
/**
*/
export class InvalidDocumentError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidDocumentError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invaliddocumenterror_free(ptr);
    }
    /**
    * @param {any} raw_document
    * @param {any[]} errors
    */
    constructor(raw_document, errors) {
        const ptr0 = passArrayJsValueToWasm0(errors, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.invaliddocumenterror_new(addHeapObject(raw_document), ptr0, len0);
        return InvalidDocumentError.__wrap(ret);
    }
    /**
    * @returns {any[]}
    */
    getErrors() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invaliddocumenterror_getErrors(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 4);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getRawDocument() {
        const ret = wasm.invaliddocumenterror_getRawDocument(this.ptr);
        return takeObject(ret);
    }
}
/**
*/
export class InvalidDocumentRevisionError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidDocumentRevisionError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invaliddocumentrevisionerror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getDocumentId() {
        const ret = wasm.invaliddocumentrevisionerror_getDocumentId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {bigint | undefined}
    */
    getCurrentRevision() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invaliddocumentrevisionerror_getCurrentRevision(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r2 = getBigInt64Memory0()[retptr / 8 + 1];
            return r0 === 0 ? undefined : BigInt.asUintN(64, r2);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invaliddocumentrevisionerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invaliddocumentrevisionerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invaliddocumentrevisionerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidDocumentTransitionActionError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidDocumentTransitionActionError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invaliddocumenttransitionactionerror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getAction() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invaliddocumenttransitionactionerror_getAction(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invaliddocumenttransitionactionerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invaliddocumenttransitionactionerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invaliddocumenttransitionactionerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidDocumentTransitionIdError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidDocumentTransitionIdError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invaliddocumenttransitioniderror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getExpectedId() {
        const ret = wasm.invaliddocumenttransitioniderror_getExpectedId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getInvalidId() {
        const ret = wasm.invaliddocumenttransitioniderror_getInvalidId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invaliddocumenttransitioniderror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invaliddocumenttransitioniderror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invaliddocumenttransitioniderror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidDocumentTypeError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidDocumentTypeError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invaliddocumenttypeerror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getType() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invaliddocumenttypeerror_getType(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    getDataContractId() {
        const ret = wasm.invaliddocumenttypeerror_getDataContractId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invaliddocumenttypeerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invaliddocumenttypeerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invaliddocumenttypeerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidDocumentTypeInDataContractError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidDocumentTypeInDataContractError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invaliddocumenttypeindatacontracterror_free(ptr);
    }
    /**
    * @param {string} doc_type
    * @param {any} data_contract_id
    */
    constructor(doc_type, data_contract_id) {
        const ptr0 = passStringToWasm0(doc_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.invaliddocumenttypeindatacontracterror_new(ptr0, len0, addHeapObject(data_contract_id));
        return InvalidDocumentTypeInDataContractError.__wrap(ret);
    }
    /**
    * @returns {string}
    */
    getType() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invaliddocumenttypeindatacontracterror_getType(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    getDataContractId() {
        const ret = wasm.invaliddocumenttypeindatacontracterror_getDataContractId(this.ptr);
        return takeObject(ret);
    }
}
/**
*/
export class InvalidIdentifierError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidIdentifierError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalididentifiererror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getIdentifierName() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentifiererror_getIdentifierName(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getIdentifierError() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentifiererror_getIdentifierError(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalididentifiererror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentifiererror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentifiererror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidIdentityAssetLockTransactionError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidIdentityAssetLockTransactionError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalididentityassetlocktransactionerror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getErrorMessage() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentityassetlocktransactionerror_getErrorMessage(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalididentityassetlocktransactionerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentityassetlocktransactionerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentityassetlocktransactionerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidIdentityAssetLockTransactionOutputError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidIdentityAssetLockTransactionOutputError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalididentityassetlocktransactionoutputerror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getOutputIndex() {
        const ret = wasm.invalididentityassetlocktransactionoutputerror_getOutputIndex(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalididentityassetlocktransactionoutputerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentityassetlocktransactionoutputerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentityassetlocktransactionoutputerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidIdentityCreditWithdrawalTransitionCoreFeeError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidIdentityCreditWithdrawalTransitionCoreFeeError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalididentitycreditwithdrawaltransitioncorefeeerror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getCoreFee() {
        const ret = wasm.invalididentitycreditwithdrawaltransitioncorefeeerror_getCoreFee(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalididentitycreditwithdrawaltransitioncorefeeerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentitycreditwithdrawaltransitioncorefeeerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentitycreditwithdrawaltransitioncorefeeerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidIdentityCreditWithdrawalTransitionOutputScriptError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidIdentityCreditWithdrawalTransitionOutputScriptError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalididentitycreditwithdrawaltransitionoutputscripterror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalididentitycreditwithdrawaltransitionoutputscripterror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentitycreditwithdrawaltransitionoutputscripterror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentitycreditwithdrawaltransitionoutputscripterror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidIdentityError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidIdentityError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalididentityerror_free(ptr);
    }
    /**
    * @returns {any[]}
    */
    getErrors() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentityerror_getErrors(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 4);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getRawIdentity() {
        const ret = wasm.invalididentityerror_getRawIdentity(this.ptr);
        return takeObject(ret);
    }
}
/**
*/
export class InvalidIdentityKeySignatureError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidIdentityKeySignatureError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalididentitykeysignatureerror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getPublicKeyId() {
        const ret = wasm.invalididentitykeysignatureerror_getPublicKeyId(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalididentitykeysignatureerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentitykeysignatureerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentitykeysignatureerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidIdentityPublicKeyDataError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidIdentityPublicKeyDataError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalididentitypublickeydataerror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getPublicKeyId() {
        const ret = wasm.invalididentitypublickeydataerror_getPublicKeyId(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    getValidationError() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentitypublickeydataerror_getValidationError(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalididentitypublickeydataerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentitypublickeydataerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentitypublickeydataerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidIdentityPublicKeyIdError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidIdentityPublicKeyIdError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalididentitypublickeyiderror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getId() {
        const ret = wasm.invalididentitypublickeyiderror_getId(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalididentitypublickeyiderror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentitypublickeyiderror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentitypublickeyiderror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidIdentityPublicKeySecurityLevelError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidIdentityPublicKeySecurityLevelError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalididentitypublickeysecuritylevelerror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getPublicKeyId() {
        const ret = wasm.invalididentitypublickeysecuritylevelerror_getPublicKeyId(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getPublicKeyPurpose() {
        const ret = wasm.invalididentitypublickeysecuritylevelerror_getPublicKeyPurpose(this.ptr);
        return ret;
    }
    /**
    * @returns {number}
    */
    getPublicKeySecurityLevel() {
        const ret = wasm.invalididentitypublickeysecuritylevelerror_getPublicKeySecurityLevel(this.ptr);
        return ret;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalididentitypublickeysecuritylevelerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentitypublickeysecuritylevelerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentitypublickeysecuritylevelerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidIdentityPublicKeyTypeError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidIdentityPublicKeyTypeError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalididentitypublickeytypeerror_free(ptr);
    }
    /**
    * @param {number} key_type
    */
    constructor(key_type) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentitypublickeytypeerror_new(retptr, key_type);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return InvalidIdentityPublicKeyTypeError.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {number}
    */
    getPublicKeyType() {
        const ret = wasm.invalididentitypublickeytypeerror_getPublicKeyType(this.ptr);
        return ret;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalididentitypublickeytypeerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentitypublickeytypeerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentitypublickeytypeerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidIdentityRevisionError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidIdentityRevisionError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalididentityrevisionerror_free(ptr);
    }
    /**
    * @returns {any}
    */
    getIdentityId() {
        const ret = wasm.invalididentityrevisionerror_getIdentityId(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCurrentRevision() {
        const ret = wasm.invalididentityrevisionerror_getCurrentRevision(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalididentityrevisionerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentityrevisionerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalididentityrevisionerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidIndexPropertyTypeError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidIndexPropertyTypeError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalidindexpropertytypeerror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getDocumentType() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidindexpropertytypeerror_getDocumentType(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getIndexName() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidindexpropertytypeerror_getIndexName(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getPropertyName() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidindexpropertytypeerror_getPropertyName(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getPropertyType() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidindexpropertytypeerror_getPropertyType(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalidindexpropertytypeerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidindexpropertytypeerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidindexpropertytypeerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidIndexedPropertyConstraintError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidIndexedPropertyConstraintError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalidindexedpropertyconstrainterror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getDocumentType() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidindexedpropertyconstrainterror_getDocumentType(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getIndexName() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidindexedpropertyconstrainterror_getIndexName(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getPropertyName() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidindexedpropertyconstrainterror_getPropertyName(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getConstraintName() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidindexedpropertyconstrainterror_getConstraintName(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getReason() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidindexedpropertyconstrainterror_getReason(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalidindexedpropertyconstrainterror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidindexedpropertyconstrainterror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidindexedpropertyconstrainterror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidInitialRevisionError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidInitialRevisionError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalidinitialrevisionerror_free(ptr);
    }
    /**
    * @param {ExtendedDocument} document
    */
    constructor(document) {
        _assertClass(document, ExtendedDocument);
        var ptr0 = document.__destroy_into_raw();
        const ret = wasm.invalidinitialrevisionerror_new(ptr0);
        return InvalidInitialRevisionError.__wrap(ret);
    }
    /**
    * @returns {ExtendedDocument}
    */
    getDocument() {
        const ret = wasm.invalidinitialrevisionerror_getDocument(this.ptr);
        return ExtendedDocument.__wrap(ret);
    }
}
/**
*/
export class InvalidInstantAssetLockProofError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidInstantAssetLockProofError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalidinstantassetlockprooferror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalidinstantassetlockprooferror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidinstantassetlockprooferror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidinstantassetlockprooferror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidInstantAssetLockProofSignatureError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidInstantAssetLockProofSignatureError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalidinstantassetlockproofsignatureerror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalidinstantassetlockproofsignatureerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidinstantassetlockproofsignatureerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidinstantassetlockproofsignatureerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidJsonSchemaRefError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidJsonSchemaRefError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalidjsonschemareferror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getRefError() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidjsonschemareferror_getRefError(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalidjsonschemareferror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidjsonschemareferror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidjsonschemareferror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidSignaturePublicKeySecurityLevelError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidSignaturePublicKeySecurityLevelError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalidsignaturepublickeysecuritylevelerror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getPublicKeySecurityLevel() {
        const ret = wasm.invalidsignaturepublickeysecuritylevelerror_getPublicKeySecurityLevel(this.ptr);
        return ret;
    }
    /**
    * @returns {number}
    */
    getKeySecurityLevelRequirement() {
        const ret = wasm.invalidsignaturepublickeysecuritylevelerror_getKeySecurityLevelRequirement(this.ptr);
        return ret;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalidsignaturepublickeysecuritylevelerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidsignaturepublickeysecuritylevelerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidsignaturepublickeysecuritylevelerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidStateTransitionError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidStateTransitionError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalidstatetransitionerror_free(ptr);
    }
    /**
    * @param {any[]} error_buffers
    * @param {any} raw_state_transition
    */
    constructor(error_buffers, raw_state_transition) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArrayJsValueToWasm0(error_buffers, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.invalidstatetransitionerror_new_wasm(retptr, ptr0, len0, addHeapObject(raw_state_transition));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return InvalidStateTransitionError.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any[]}
    */
    getErrors() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidstatetransitionerror_getErrors(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 4);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getRawStateTransition() {
        const ret = wasm.invalidstatetransitionerror_getRawStateTransition(this.ptr);
        return takeObject(ret);
    }
}
/**
*/
export class InvalidStateTransitionSignatureError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidStateTransitionSignatureError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalidstatetransitionsignatureerror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalidstatetransitionsignatureerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidstatetransitionsignatureerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidstatetransitionsignatureerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class InvalidStateTransitionTypeError {

    static __wrap(ptr) {
        const obj = Object.create(InvalidStateTransitionTypeError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_invalidstatetransitiontypeerror_free(ptr);
    }
    /**
    * @param {number} transition_type
    */
    constructor(transition_type) {
        const ret = wasm.invalidstatetransitiontypeerror_new(transition_type);
        return InvalidStateTransitionTypeError.__wrap(ret);
    }
    /**
    * @returns {number}
    */
    getType() {
        const ret = wasm.invalidstatetransitiontypeerror_getType(this.ptr);
        return ret;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.invalidstatetransitiontypeerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidstatetransitiontypeerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.invalidstatetransitiontypeerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class JsonSchemaCompilationError {

    static __wrap(ptr) {
        const obj = Object.create(JsonSchemaCompilationError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_jsonschemacompilationerror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getError() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsonschemacompilationerror_getError(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.jsonschemacompilationerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsonschemacompilationerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsonschemacompilationerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class JsonSchemaError {

    static __wrap(ptr) {
        const obj = Object.create(JsonSchemaError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    toJSON() {
        return {
            message: this.message,
        };
    }

    toString() {
        return JSON.stringify(this);
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_jsonschemaerror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getKeyword() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsonschemaerror_getKeyword(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getInstancePath() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsonschemaerror_getInstancePath(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getSchemaPath() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsonschemaerror_getSchemaPath(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getPropertyName() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsonschemaerror_getPropertyName(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    getParams() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsonschemaerror_getParams(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.jsonschemaerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    toString() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsonschemaerror_toString(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsonschemaerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsonschemaerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class JsonSchemaValidator {

    static __wrap(ptr) {
        const obj = Object.create(JsonSchemaValidator.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_jsonschemavalidator_free(ptr);
    }
    /**
    * @param {any} schema_js
    * @param {any} definitions
    */
    constructor(schema_js, definitions) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsonschemavalidator_new(retptr, addBorrowedObject(schema_js), addBorrowedObject(definitions));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return JsonSchemaValidator.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            heap[stack_pointer++] = undefined;
            heap[stack_pointer++] = undefined;
        }
    }
}
/**
*/
export class MaxIdentityPublicKeyLimitReachedError {

    static __wrap(ptr) {
        const obj = Object.create(MaxIdentityPublicKeyLimitReachedError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_maxidentitypublickeylimitreachederror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getMaxItems() {
        const ret = wasm.maxidentitypublickeylimitreachederror_getMaxItems(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.maxidentitypublickeylimitreachederror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.maxidentitypublickeylimitreachederror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.maxidentitypublickeylimitreachederror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class Metadata {

    static __wrap(ptr) {
        const obj = Object.create(Metadata.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_metadata_free(ptr);
    }
    /**
    * @param {any} options
    */
    constructor(options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.metadata_new(retptr, addHeapObject(options));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return Metadata.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} object
    * @returns {Metadata}
    */
    static from(object) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.metadata_from(retptr, addHeapObject(object));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return Metadata.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    toJSON() {
        const ret = wasm.metadata_toJSON(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    toObject() {
        const ret = wasm.metadata_toObject(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {number}
    */
    getBlockHeight() {
        const ret = wasm.metadata_getBlockHeight(this.ptr);
        return ret;
    }
    /**
    * @returns {number}
    */
    getCoreChainLockedHeight() {
        const ret = wasm.metadata_getCoreChainLockedHeight(this.ptr);
        return ret;
    }
    /**
    * @returns {number}
    */
    getTimeMs() {
        const ret = wasm.metadata_getTimeMs(this.ptr);
        return ret;
    }
    /**
    * @returns {number}
    */
    getProtocolVersion() {
        const ret = wasm.metadata_getProtocolVersion(this.ptr);
        return ret;
    }
}
/**
*/
export class MismatchOwnerIdsError {

    static __wrap(ptr) {
        const obj = Object.create(MismatchOwnerIdsError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_mismatchowneridserror_free(ptr);
    }
    /**
    * @param {any[]} documents
    */
    constructor(documents) {
        const ptr0 = passArrayJsValueToWasm0(documents, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.mismatchowneridserror_new(ptr0, len0);
        return MismatchOwnerIdsError.__wrap(ret);
    }
    /**
    * @returns {any[]}
    */
    getDocuments() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.mismatchowneridserror_getDocuments(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 4);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class MissingDataContractIdError {

    static __wrap(ptr) {
        const obj = Object.create(MissingDataContractIdError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_missingdatacontractiderror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.missingdatacontractiderror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.missingdatacontractiderror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.missingdatacontractiderror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class MissingDocumentTransitionActionError {

    static __wrap(ptr) {
        const obj = Object.create(MissingDocumentTransitionActionError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_missingdocumenttransitionactionerror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.missingdocumenttransitionactionerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.missingdocumenttransitionactionerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.missingdocumenttransitionactionerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class MissingDocumentTransitionTypeError {

    static __wrap(ptr) {
        const obj = Object.create(MissingDocumentTransitionTypeError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_missingdocumenttransitiontypeerror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.missingdocumenttransitiontypeerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.missingdocumenttransitiontypeerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.missingdocumenttransitiontypeerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class MissingDocumentTypeError {

    static __wrap(ptr) {
        const obj = Object.create(MissingDocumentTypeError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_missingdocumenttypeerror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.missingdocumenttypeerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.missingdocumenttypeerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.missingdocumenttypeerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class MissingMasterPublicKeyError {

    static __wrap(ptr) {
        const obj = Object.create(MissingMasterPublicKeyError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_missingmasterpublickeyerror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.missingmasterpublickeyerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.missingmasterpublickeyerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.missingmasterpublickeyerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class MissingPublicKeyError {

    static __wrap(ptr) {
        const obj = Object.create(MissingPublicKeyError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_missingpublickeyerror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getPublicKeyId() {
        const ret = wasm.missingpublickeyerror_getPublicKeyId(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.missingpublickeyerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.missingpublickeyerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.missingpublickeyerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class MissingStateTransitionTypeError {

    static __wrap(ptr) {
        const obj = Object.create(MissingStateTransitionTypeError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_missingstatetransitiontypeerror_free(ptr);
    }
    /**
    */
    constructor() {
        const ret = wasm.missingstatetransitiontypeerror_new();
        return MissingStateTransitionTypeError.__wrap(ret);
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.missingstatetransitiontypeerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.missingstatetransitiontypeerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.missingstatetransitiontypeerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class NoDocumentsSuppliedError {

    static __wrap(ptr) {
        const obj = Object.create(NoDocumentsSuppliedError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_nodocumentssuppliederror_free(ptr);
    }
    /**
    */
    constructor() {
        const ret = wasm.nodocumentssuppliederror_new();
        return NoDocumentsSuppliedError.__wrap(ret);
    }
}
/**
*/
export class NonConsensusErrorWasm {

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_nonconsensuserrorwasm_free(ptr);
    }
}
/**
*/
export class NotImplementedIdentityCreditWithdrawalTransitionPoolingError {

    static __wrap(ptr) {
        const obj = Object.create(NotImplementedIdentityCreditWithdrawalTransitionPoolingError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_notimplementedidentitycreditwithdrawaltransitionpoolingerror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getPooling() {
        const ret = wasm.notimplementedidentitycreditwithdrawaltransitionpoolingerror_getPooling(this.ptr);
        return ret;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.notimplementedidentitycreditwithdrawaltransitionpoolingerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.notimplementedidentitycreditwithdrawaltransitionpoolingerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.notimplementedidentitycreditwithdrawaltransitionpoolingerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class Operation {

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_operation_free(ptr);
    }
}
/**
*/
export class PlatformValueError {

    static __wrap(ptr) {
        const obj = Object.create(PlatformValueError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    toJSON() {
        return {
        };
    }

    toString() {
        return JSON.stringify(this);
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_platformvalueerror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getMessage() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.platformvalueerror_getMessage(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    toString() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.platformvalueerror_toString(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
}
/**
*/
export class PreCalculatedOperation {

    static __wrap(ptr) {
        const obj = Object.create(PreCalculatedOperation.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_precalculatedoperation_free(ptr);
    }
    /**
    * @param {any} storage_cost
    * @param {any} processing_cost
    * @param {Array<any>} js_fee_refunds
    */
    constructor(storage_cost, processing_cost, js_fee_refunds) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.precalculatedoperation_new(retptr, addHeapObject(storage_cost), addHeapObject(processing_cost), addHeapObject(js_fee_refunds));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return PreCalculatedOperation.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {DummyFeesResult} dummy_fee_result
    * @returns {PreCalculatedOperation}
    */
    static fromFee(dummy_fee_result) {
        _assertClass(dummy_fee_result, DummyFeesResult);
        const ret = wasm.precalculatedoperation_fromFee(dummy_fee_result.ptr);
        return PreCalculatedOperation.__wrap(ret);
    }
    /**
    * @returns {bigint}
    */
    getProcessingCost() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.precalculatedoperation_getProcessingCost(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {bigint}
    */
    getStorageCost() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.precalculatedoperation_getStorageCost(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {Array<any> | undefined}
    */
    get refunds() {
        const ret = wasm.precalculatedoperation_refunds(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {Array<any> | undefined}
    */
    refunds_as_objects() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.precalculatedoperation_refunds_as_objects(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    toJSON() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.precalculatedoperation_toJSON(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class ProtocolVersionParsingError {

    static __wrap(ptr) {
        const obj = Object.create(ProtocolVersionParsingError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_protocolversionparsingerror_free(ptr);
    }
    /**
    * @param {string} parsing_error
    */
    constructor(parsing_error) {
        const ptr0 = passStringToWasm0(parsing_error, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.protocolversionparsingerror_new(ptr0, len0);
        return ProtocolVersionParsingError.__wrap(ret);
    }
    /**
    * @returns {string}
    */
    getParsingError() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.protocolversionparsingerror_getParsingError(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.protocolversionparsingerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.protocolversionparsingerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.protocolversionparsingerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
}
/**
*/
export class ProtocolVersionValidator {

    static __wrap(ptr) {
        const obj = Object.create(ProtocolVersionValidator.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_protocolversionvalidator_free(ptr);
    }
    /**
    * @param {any} options
    */
    constructor(options) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.protocolversionvalidator_new(retptr, addHeapObject(options));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return ProtocolVersionValidator.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {number} version
    * @returns {ValidationResult}
    */
    validate(version) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.protocolversionvalidator_validate(retptr, this.ptr, version);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return ValidationResult.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class PublicKeyIsDisabledError {

    static __wrap(ptr) {
        const obj = Object.create(PublicKeyIsDisabledError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_publickeyisdisablederror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getPublicKeyId() {
        const ret = wasm.publickeyisdisablederror_getPublicKeyId(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.publickeyisdisablederror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.publickeyisdisablederror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.publickeyisdisablederror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class PublicKeySecurityLevelNotMetError {

    static __wrap(ptr) {
        const obj = Object.create(PublicKeySecurityLevelNotMetError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_publickeysecuritylevelnotmeterror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getPublicKeySecurityLevel() {
        const ret = wasm.publickeysecuritylevelnotmeterror_getPublicKeySecurityLevel(this.ptr);
        return ret;
    }
    /**
    * @returns {number}
    */
    getKeySecurityLevelRequirement() {
        const ret = wasm.publickeysecuritylevelnotmeterror_getKeySecurityLevelRequirement(this.ptr);
        return ret;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.publickeysecuritylevelnotmeterror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.publickeysecuritylevelnotmeterror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.publickeysecuritylevelnotmeterror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class PublicKeyValidationError {

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_publickeyvalidationerror_free(ptr);
    }
    /**
    * @returns {any}
    */
    get message() {
        const ret = wasm.publickeyvalidationerror_message(this.ptr);
        return takeObject(ret);
    }
}
/**
*/
export class PublicKeysSignaturesValidator {

    static __wrap(ptr) {
        const obj = Object.create(PublicKeysSignaturesValidator.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_publickeyssignaturesvalidator_free(ptr);
    }
    /**
    * @param {any} bls
    */
    constructor(bls) {
        const ret = wasm.publickeyssignaturesvalidator_new(addHeapObject(bls));
        return PublicKeysSignaturesValidator.__wrap(ret);
    }
    /**
    * @param {any} raw_state_transition
    * @param {any[]} raw_public_keys
    * @returns {ValidationResult}
    */
    validate(raw_state_transition, raw_public_keys) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passArrayJsValueToWasm0(raw_public_keys, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.publickeyssignaturesvalidator_validate(retptr, this.ptr, addHeapObject(raw_state_transition), ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return ValidationResult.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class PublicKeysValidator {

    static __wrap(ptr) {
        const obj = Object.create(PublicKeysValidator.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_publickeysvalidator_free(ptr);
    }
    /**
    * @param {any} adapter
    */
    constructor(adapter) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.publickeysvalidator_new(retptr, addHeapObject(adapter));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return PublicKeysValidator.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Array<any>} public_keys
    * @returns {ValidationResult}
    */
    validateKeys(public_keys) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.publickeysvalidator_validateKeys(retptr, this.ptr, addHeapObject(public_keys));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return ValidationResult.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} public_key
    * @returns {ValidationResult}
    */
    validatePublicKeyStructure(public_key) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.publickeysvalidator_validatePublicKeyStructure(retptr, this.ptr, addHeapObject(public_key));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return ValidationResult.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {Array<any>} public_keys
    * @returns {ValidationResult}
    */
    validateKeysInStateTransition(public_keys) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.publickeysvalidator_validateKeysInStateTransition(retptr, this.ptr, addHeapObject(public_keys));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return ValidationResult.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class ReadOperation {

    static __wrap(ptr) {
        const obj = Object.create(ReadOperation.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_readoperation_free(ptr);
    }
    /**
    * @param {any} value_size
    */
    constructor(value_size) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.readoperation_new(retptr, addHeapObject(value_size));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return ReadOperation.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {bigint}
    */
    get processingCost() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.readoperation_processingCost(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {bigint}
    */
    get storageCost() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.readoperation_storageCost(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {Array<any> | undefined}
    */
    get refunds() {
        const ret = wasm.readoperation_refunds(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    toJSON() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.readoperation_toJSON(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class Refunds {

    static __wrap(ptr) {
        const obj = Object.create(Refunds.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_refunds_free(ptr);
    }
    /**
    * @returns {any}
    */
    get identifier() {
        const ret = wasm.refunds_identifier(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {Map<any, any>}
    */
    get credits_per_epoch() {
        const ret = wasm.refunds_credits_per_epoch(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    toObject() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.refunds_toObject(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class RevisionAbsentError {

    static __wrap(ptr) {
        const obj = Object.create(RevisionAbsentError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_revisionabsenterror_free(ptr);
    }
    /**
    * @param {ExtendedDocument} extended_document
    */
    constructor(extended_document) {
        _assertClass(extended_document, ExtendedDocument);
        var ptr0 = extended_document.__destroy_into_raw();
        const ret = wasm.revisionabsenterror_new(ptr0);
        return RevisionAbsentError.__wrap(ret);
    }
    /**
    * @returns {ExtendedDocument}
    */
    getDocument() {
        const ret = wasm.revisionabsenterror_getDocument(this.ptr);
        return ExtendedDocument.__wrap(ret);
    }
}
/**
*/
export class SerializedObjectParsingError {

    static __wrap(ptr) {
        const obj = Object.create(SerializedObjectParsingError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_serializedobjectparsingerror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getParsingError() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.serializedobjectparsingerror_getParsingError(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.serializedobjectparsingerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.serializedobjectparsingerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.serializedobjectparsingerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class SignatureVerificationOperation {

    static __wrap(ptr) {
        const obj = Object.create(SignatureVerificationOperation.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_signatureverificationoperation_free(ptr);
    }
    /**
    * @param {number} signature_type
    */
    constructor(signature_type) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.signatureverificationoperation_new(retptr, signature_type);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return SignatureVerificationOperation.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {bigint}
    */
    getProcessingCost() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.signatureverificationoperation_getProcessingCost(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {bigint}
    */
    getStorageCost() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.signatureverificationoperation_getStorageCost(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {Array<any> | undefined}
    */
    get refunds() {
        const ret = wasm.signatureverificationoperation_refunds(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    toJSON() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.signatureverificationoperation_toJSON(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class StateTransitionExecutionContext {

    static __wrap(ptr) {
        const obj = Object.create(StateTransitionExecutionContext.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_statetransitionexecutioncontext_free(ptr);
    }
    /**
    */
    constructor() {
        const ret = wasm.statetransitionexecutioncontext_new();
        return StateTransitionExecutionContext.__wrap(ret);
    }
    /**
    */
    enableDryRun() {
        wasm.statetransitionexecutioncontext_enableDryRun(this.ptr);
    }
    /**
    */
    disableDryRun() {
        wasm.statetransitionexecutioncontext_disableDryRun(this.ptr);
    }
    /**
    * @returns {boolean}
    */
    isDryRun() {
        const ret = wasm.statetransitionexecutioncontext_isDryRun(this.ptr);
        return ret !== 0;
    }
    /**
    * @param {any} operation
    */
    addOperation(operation) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.statetransitionexecutioncontext_addOperation(retptr, this.ptr, addHeapObject(operation));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            if (r1) {
                throw takeObject(r0);
            }
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any[]}
    */
    getOperations() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.statetransitionexecutioncontext_getOperations(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 4);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    */
    clearDryOperations() {
        wasm.statetransitionexecutioncontext_clearDryOperations(this.ptr);
    }
}
/**
*/
export class StateTransitionFacade {

    static __wrap(ptr) {
        const obj = Object.create(StateTransitionFacade.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_statetransitionfacade_free(ptr);
    }
    /**
    * @param {any} raw_state_transition
    * @param {any} options
    * @returns {Promise<any>}
    */
    createFromObject(raw_state_transition, options) {
        const ret = wasm.statetransitionfacade_createFromObject(this.ptr, addHeapObject(raw_state_transition), addHeapObject(options));
        return takeObject(ret);
    }
    /**
    * @param {Uint8Array} state_transition_buffer
    * @param {any} options
    * @returns {Promise<any>}
    */
    createFromBuffer(state_transition_buffer, options) {
        const ptr0 = passArray8ToWasm0(state_transition_buffer, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.statetransitionfacade_createFromBuffer(this.ptr, ptr0, len0, addHeapObject(options));
        return takeObject(ret);
    }
    /**
    * @param {any} raw_state_transition
    * @param {any} options
    * @returns {Promise<ValidationResult>}
    */
    validate(raw_state_transition, options) {
        const ret = wasm.statetransitionfacade_validate(this.ptr, addHeapObject(raw_state_transition), addHeapObject(options));
        return takeObject(ret);
    }
    /**
    * @param {any} raw_state_transition
    * @returns {Promise<ValidationResult>}
    */
    validateBasic(raw_state_transition) {
        const ret = wasm.statetransitionfacade_validateBasic(this.ptr, addHeapObject(raw_state_transition));
        return takeObject(ret);
    }
    /**
    * @param {any} raw_state_transition
    * @returns {Promise<ValidationResult>}
    */
    validateSignature(raw_state_transition) {
        const ret = wasm.statetransitionfacade_validateSignature(this.ptr, addHeapObject(raw_state_transition));
        return takeObject(ret);
    }
    /**
    * @param {any} raw_state_transition
    * @returns {Promise<ValidationResult>}
    */
    validateFee(raw_state_transition) {
        const ret = wasm.statetransitionfacade_validateFee(this.ptr, addHeapObject(raw_state_transition));
        return takeObject(ret);
    }
    /**
    * @param {any} raw_state_transition
    * @returns {Promise<ValidationResult>}
    */
    validateState(raw_state_transition) {
        const ret = wasm.statetransitionfacade_validateState(this.ptr, addHeapObject(raw_state_transition));
        return takeObject(ret);
    }
    /**
    * @param {any} state_transition_js
    * @returns {Promise<any>}
    */
    apply(state_transition_js) {
        const ret = wasm.statetransitionfacade_apply(this.ptr, addHeapObject(state_transition_js));
        return takeObject(ret);
    }
}
/**
*/
export class StateTransitionFactory {

    static __wrap(ptr) {
        const obj = Object.create(StateTransitionFactory.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_statetransitionfactory_free(ptr);
    }
    /**
    * @param {any} state_repository
    * @param {any} bls_adapter
    */
    constructor(state_repository, bls_adapter) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.statetransitionfactory_new(retptr, addHeapObject(state_repository), addHeapObject(bls_adapter));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return StateTransitionFactory.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @param {any} state_transition_object
    * @param {any} options
    * @returns {Promise<any>}
    */
    createFromObject(state_transition_object, options) {
        const ret = wasm.statetransitionfactory_createFromObject(this.ptr, addHeapObject(state_transition_object), addHeapObject(options));
        return takeObject(ret);
    }
    /**
    * @param {Uint8Array} buffer
    * @param {any} options
    * @returns {Promise<any>}
    */
    createFromBuffer(buffer, options) {
        const ptr0 = passArray8ToWasm0(buffer, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.statetransitionfactory_createFromBuffer(this.ptr, ptr0, len0, addHeapObject(options));
        return takeObject(ret);
    }
}
/**
*/
export class StateTransitionKeySignatureValidator {

    static __wrap(ptr) {
        const obj = Object.create(StateTransitionKeySignatureValidator.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_statetransitionkeysignaturevalidator_free(ptr);
    }
    /**
    * @param {any} state_repository
    */
    constructor(state_repository) {
        const ret = wasm.statetransitionkeysignaturevalidator_new(addHeapObject(state_repository));
        return StateTransitionKeySignatureValidator.__wrap(ret);
    }
    /**
    * @param {any} state_transition
    * @returns {Promise<ValidationResult>}
    */
    validate(state_transition) {
        const ret = wasm.statetransitionkeysignaturevalidator_validate(this.ptr, addHeapObject(state_transition));
        return takeObject(ret);
    }
}
/**
*/
export class StateTransitionMaxSizeExceededError {

    static __wrap(ptr) {
        const obj = Object.create(StateTransitionMaxSizeExceededError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_statetransitionmaxsizeexceedederror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getActualSizeKBytes() {
        const ret = wasm.statetransitionmaxsizeexceedederror_getActualSizeKBytes(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getMaxSizeKBytes() {
        const ret = wasm.statetransitionmaxsizeexceedederror_getMaxSizeKBytes(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.statetransitionmaxsizeexceedederror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.statetransitionmaxsizeexceedederror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.statetransitionmaxsizeexceedederror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class SystemPropertyIndexAlreadyPresentError {

    static __wrap(ptr) {
        const obj = Object.create(SystemPropertyIndexAlreadyPresentError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_systempropertyindexalreadypresenterror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getDocumentType() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.systempropertyindexalreadypresenterror_getDocumentType(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getIndexName() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.systempropertyindexalreadypresenterror_getIndexName(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getPropertyName() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.systempropertyindexalreadypresenterror_getPropertyName(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.systempropertyindexalreadypresenterror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.systempropertyindexalreadypresenterror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.systempropertyindexalreadypresenterror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class TryingToReplaceImmutableDocumentError {

    static __wrap(ptr) {
        const obj = Object.create(TryingToReplaceImmutableDocumentError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_tryingtoreplaceimmutabledocumenterror_free(ptr);
    }
    /**
    * @param {ExtendedDocument} extended_document
    */
    constructor(extended_document) {
        _assertClass(extended_document, ExtendedDocument);
        var ptr0 = extended_document.__destroy_into_raw();
        const ret = wasm.tryingtoreplaceimmutabledocumenterror_new(ptr0);
        return TryingToReplaceImmutableDocumentError.__wrap(ret);
    }
}
/**
*/
export class UndefinedIndexPropertyError {

    static __wrap(ptr) {
        const obj = Object.create(UndefinedIndexPropertyError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_undefinedindexpropertyerror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getDocumentType() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.undefinedindexpropertyerror_getDocumentType(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getIndexName() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.undefinedindexpropertyerror_getIndexName(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {string}
    */
    getPropertyName() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.undefinedindexpropertyerror_getPropertyName(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.undefinedindexpropertyerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.undefinedindexpropertyerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.undefinedindexpropertyerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class UniqueIndicesLimitReachedError {

    static __wrap(ptr) {
        const obj = Object.create(UniqueIndicesLimitReachedError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_uniqueindiceslimitreachederror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getDocumentType() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.uniqueindiceslimitreachederror_getDocumentType(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getIndexLimit() {
        const ret = wasm.uniqueindiceslimitreachederror_getIndexLimit(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.uniqueindiceslimitreachederror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.uniqueindiceslimitreachederror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.uniqueindiceslimitreachederror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class UnknownAssetLockProofTypeError {

    static __wrap(ptr) {
        const obj = Object.create(UnknownAssetLockProofTypeError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_unknownassetlockprooftypeerror_free(ptr);
    }
    /**
    * @returns {number | undefined}
    */
    getType() {
        const ret = wasm.unknownassetlockprooftypeerror_getType(this.ptr);
        return ret === 0xFFFFFF ? undefined : ret;
    }
}
/**
*/
export class UnsupportedProtocolVersionError {

    static __wrap(ptr) {
        const obj = Object.create(UnsupportedProtocolVersionError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_unsupportedprotocolversionerror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getParsedProtocolVersion() {
        const ret = wasm.unsupportedprotocolversionerror_getParsedProtocolVersion(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getLatestVersion() {
        const ret = wasm.unsupportedprotocolversionerror_getLatestVersion(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.unsupportedprotocolversionerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.unsupportedprotocolversionerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.unsupportedprotocolversionerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class ValidationResult {

    static __wrap(ptr) {
        const obj = Object.create(ValidationResult.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_validationresult_free(ptr);
    }
    /**
    * @param {any[] | undefined} errors_option
    */
    constructor(errors_option) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            var ptr0 = isLikeNone(errors_option) ? 0 : passArrayJsValueToWasm0(errors_option, wasm.__wbindgen_malloc);
            var len0 = WASM_VECTOR_LEN;
            wasm.validationresult_new(retptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return ValidationResult.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * This is just a test method - doesn't need to be in the resulted binding. Please
    * remove before shipping
    * @returns {(string)[]}
    */
    errorsText() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.validationresult_errorsText(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 4);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {boolean}
    */
    isValid() {
        const ret = wasm.validationresult_isValid(this.ptr);
        return ret !== 0;
    }
    /**
    * @returns {any[]}
    */
    getErrors() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.validationresult_getErrors(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var v0 = getArrayJsValueFromWasm0(r0, r1).slice();
            wasm.__wbindgen_free(r0, r1 * 4);
            return v0;
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * @returns {any}
    */
    getData() {
        const ret = wasm.validationresult_getData(this.ptr);
        return takeObject(ret);
    }
    /**
    * @returns {any}
    */
    getFirstError() {
        const ret = wasm.validationresult_getFirstError(this.ptr);
        return takeObject(ret);
    }
    /**
    * @param {any} error_buffer
    * @returns {any}
    */
    addError(error_buffer) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.validationresult_addError(retptr, this.ptr, addHeapObject(error_buffer));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class ValueError {

    static __wrap(ptr) {
        const obj = Object.create(ValueError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    toJSON() {
        return {
            message: this.message,
        };
    }

    toString() {
        return JSON.stringify(this);
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_valueerror_free(ptr);
    }
    /**
    * @returns {string}
    */
    getMessage() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.valueerror_getMessage(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.valueerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.valueerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.valueerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}
/**
*/
export class WrongPublicKeyPurposeError {

    static __wrap(ptr) {
        const obj = Object.create(WrongPublicKeyPurposeError.prototype);
        obj.ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.ptr;
        this.ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wrongpublickeypurposeerror_free(ptr);
    }
    /**
    * @returns {number}
    */
    getPublicKeyPurpose() {
        const ret = wasm.wrongpublickeypurposeerror_getPublicKeyPurpose(this.ptr);
        return ret;
    }
    /**
    * @returns {number}
    */
    getKeyPurposeRequirement() {
        const ret = wasm.wrongpublickeypurposeerror_getKeyPurposeRequirement(this.ptr);
        return ret;
    }
    /**
    * @returns {number}
    */
    getCode() {
        const ret = wasm.wrongpublickeypurposeerror_getCode(this.ptr);
        return ret >>> 0;
    }
    /**
    * @returns {string}
    */
    get message() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.wrongpublickeypurposeerror_message(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(r0, r1);
        }
    }
    /**
    * @returns {any}
    */
    serialize() {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.wrongpublickeypurposeerror_serialize(retptr, this.ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
}

async function load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);

            } catch (e) {
                if (module.headers.get('Content-Type') != 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else {
                    throw e;
                }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);

    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };

        } else {
            return instance;
        }
    }
}

function getImports() {
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbindgen_error_new = function(arg0, arg1) {
        const ret = new Error(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_is_undefined = function(arg0) {
        const ret = getObject(arg0) === undefined;
        return ret;
    };
    imports.wbg.__wbindgen_ge = function(arg0, arg1) {
        const ret = getObject(arg0) >= getObject(arg1);
        return ret;
    };
    imports.wbg.__wbindgen_in = function(arg0, arg1) {
        const ret = getObject(arg0) in getObject(arg1);
        return ret;
    };
    imports.wbg.__wbindgen_number_get = function(arg0, arg1) {
        const obj = getObject(arg1);
        const ret = typeof(obj) === 'number' ? obj : undefined;
        getFloat64Memory0()[arg0 / 8 + 1] = isLikeNone(ret) ? 0 : ret;
        getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
    };
    imports.wbg.__wbindgen_boolean_get = function(arg0) {
        const v = getObject(arg0);
        const ret = typeof(v) === 'boolean' ? (v ? 1 : 0) : 2;
        return ret;
    };
    imports.wbg.__wbindgen_is_null = function(arg0) {
        const ret = getObject(arg0) === null;
        return ret;
    };
    imports.wbg.__wbindgen_number_new = function(arg0) {
        const ret = arg0;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
        const ret = getStringFromWasm0(arg0, arg1);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_is_falsy = function(arg0) {
        const ret = !getObject(arg0);
        return ret;
    };
    imports.wbg.__wbindgen_string_get = function(arg0, arg1) {
        const obj = getObject(arg1);
        const ret = typeof(obj) === 'string' ? obj : undefined;
        var ptr0 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    };
    imports.wbg.__wbindgen_is_bigint = function(arg0) {
        const ret = typeof(getObject(arg0)) === 'bigint';
        return ret;
    };
    imports.wbg.__wbindgen_is_object = function(arg0) {
        const val = getObject(arg0);
        const ret = typeof(val) === 'object' && val !== null;
        return ret;
    };
    imports.wbg.__wbindgen_object_clone_ref = function(arg0) {
        const ret = getObject(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_jsval_eq = function(arg0, arg1) {
        const ret = getObject(arg0) === getObject(arg1);
        return ret;
    };
    imports.wbg.__wbindgen_bigint_from_i64 = function(arg0) {
        const ret = arg0;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_bigint_from_u64 = function(arg0) {
        const ret = BigInt.asUintN(64, arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_datacontract_new = function(arg0) {
        const ret = DataContract.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_datacontractgenericerror_new = function(arg0) {
        const ret = DataContractGenericError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invaliddatacontracterror_new = function(arg0) {
        const ret = InvalidDataContractError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invaliddocumenttypeindatacontracterror_new = function(arg0) {
        const ret = InvalidDocumentTypeInDataContractError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_datacontractcreatetransition_new = function(arg0) {
        const ret = DataContractCreateTransition.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_datacontractupdatetransition_new = function(arg0) {
        const ret = DataContractUpdateTransition.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_indexproperty_new = function(arg0) {
        const ret = IndexProperty.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_datatriggerexecutionresult_new = function(arg0) {
        const ret = DataTriggerExecutionResult.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_datatrigger_new = function(arg0) {
        const ret = DataTrigger.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_documentalreadyexistserror_new = function(arg0) {
        const ret = DocumentAlreadyExistsError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_documentnorevisionerror_new = function(arg0) {
        const ret = DocumentNoRevisionError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_documentnotprovidederror_new = function(arg0) {
        const ret = DocumentNotProvidedError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalidactionerror_new = function(arg0) {
        const ret = InvalidActionError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalidactionnameerror_new = function(arg0) {
        const ret = InvalidActionNameError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invaliddocumentactionerror_new = function(arg0) {
        const ret = InvalidDocumentActionError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invaliddocumenterror_new = function(arg0) {
        const ret = InvalidDocumentError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalidinitialrevisionerror_new = function(arg0) {
        const ret = InvalidInitialRevisionError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_mismatchowneridserror_new = function(arg0) {
        const ret = MismatchOwnerIdsError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_nodocumentssuppliederror_new = function(arg0) {
        const ret = NoDocumentsSuppliedError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_revisionabsenterror_new = function(arg0) {
        const ret = RevisionAbsentError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_tryingtoreplaceimmutabledocumenterror_new = function(arg0) {
        const ret = TryingToReplaceImmutableDocumentError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_extendeddocument_new = function(arg0) {
        const ret = ExtendedDocument.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_documentcreatetransition_new = function(arg0) {
        const ret = DocumentCreateTransition.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_documentdeletetransition_new = function(arg0) {
        const ret = DocumentDeleteTransition.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_documentreplacetransition_new = function(arg0) {
        const ret = DocumentReplaceTransition.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_documenttransition_new = function(arg0) {
        const ret = DocumentTransition.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_documentsbatchtransition_new = function(arg0) {
        const ret = DocumentsBatchTransition.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_datacontracthavenewuniqueindexerror_new = function(arg0) {
        const ret = DataContractHaveNewUniqueIndexError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_datacontractimmutablepropertiesupdateerror_new = function(arg0) {
        const ret = DataContractImmutablePropertiesUpdateError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_datacontractinvalidindexdefinitionupdateerror_new = function(arg0) {
        const ret = DataContractInvalidIndexDefinitionUpdateError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_datacontractmaxdepthexceederror_new = function(arg0) {
        const ret = DataContractMaxDepthExceedError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_datacontractuniqueindiceschangederror_new = function(arg0) {
        const ret = DataContractUniqueIndicesChangedError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_duplicateindexnameerror_new = function(arg0) {
        const ret = DuplicateIndexNameError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_incompatibledatacontractschemaerror_new = function(arg0) {
        const ret = IncompatibleDataContractSchemaError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_incompatiblere2patternerror_new = function(arg0) {
        const ret = IncompatibleRe2PatternError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_duplicateindexerror_new = function(arg0) {
        const ret = DuplicateIndexError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalidcompoundindexerror_new = function(arg0) {
        const ret = InvalidCompoundIndexError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalidindexpropertytypeerror_new = function(arg0) {
        const ret = InvalidIndexPropertyTypeError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalidindexedpropertyconstrainterror_new = function(arg0) {
        const ret = InvalidIndexedPropertyConstraintError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_systempropertyindexalreadypresenterror_new = function(arg0) {
        const ret = SystemPropertyIndexAlreadyPresentError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_undefinedindexpropertyerror_new = function(arg0) {
        const ret = UndefinedIndexPropertyError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_uniqueindiceslimitreachederror_new = function(arg0) {
        const ret = UniqueIndicesLimitReachedError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invaliddatacontractiderror_new = function(arg0) {
        const ret = InvalidDataContractIdError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invaliddatacontractversionerror_new = function(arg0) {
        const ret = InvalidDataContractVersionError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalidjsonschemareferror_new = function(arg0) {
        const ret = InvalidJsonSchemaRefError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_protocolversionparsingerror_new = function(arg0) {
        const ret = ProtocolVersionParsingError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_serializedobjectparsingerror_new = function(arg0) {
        const ret = SerializedObjectParsingError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_datacontractnotpresenterror_new = function(arg0) {
        const ret = DataContractNotPresentError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_duplicatedocumenttransitionswithidserror_new = function(arg0) {
        const ret = DuplicateDocumentTransitionsWithIdsError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_duplicatedocumenttransitionswithindiceserror_new = function(arg0) {
        const ret = DuplicateDocumentTransitionsWithIndicesError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_inconsistentcompoundindexdataerror_new = function(arg0) {
        const ret = InconsistentCompoundIndexDataError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invaliddocumenttransitionactionerror_new = function(arg0) {
        const ret = InvalidDocumentTransitionActionError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invaliddocumenttransitioniderror_new = function(arg0) {
        const ret = InvalidDocumentTransitionIdError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invaliddocumenttypeerror_new = function(arg0) {
        const ret = InvalidDocumentTypeError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_missingdatacontractiderror_new = function(arg0) {
        const ret = MissingDataContractIdError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_missingdocumenttransitionactionerror_new = function(arg0) {
        const ret = MissingDocumentTransitionActionError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_missingdocumenttransitiontypeerror_new = function(arg0) {
        const ret = MissingDocumentTransitionTypeError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_missingdocumenttypeerror_new = function(arg0) {
        const ret = MissingDocumentTypeError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_duplicatedidentitypublickeyerror_new = function(arg0) {
        const ret = DuplicatedIdentityPublicKeyError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_duplicatedidentitypublickeyiderror_new = function(arg0) {
        const ret = DuplicatedIdentityPublicKeyIdError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_identityassetlockprooflockedtransactionmismatcherror_new = function(arg0) {
        const ret = IdentityAssetLockProofLockedTransactionMismatchError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_identityassetlocktransactionisnotfounderror_new = function(arg0) {
        const ret = IdentityAssetLockTransactionIsNotFoundError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_identityassetlocktransactionoutpointalreadyexistserror_new = function(arg0) {
        const ret = IdentityAssetLockTransactionOutPointAlreadyExistsError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_identityassetlocktransactionoutputnotfounderror_new = function(arg0) {
        const ret = IdentityAssetLockTransactionOutputNotFoundError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_identityinsufficientbalanceerror_new = function(arg0) {
        const ret = IdentityInsufficientBalanceError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalidassetlockproofcorechainheighterror_new = function(arg0) {
        const ret = InvalidAssetLockProofCoreChainHeightError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalidassetlockprooftransactionheighterror_new = function(arg0) {
        const ret = InvalidAssetLockProofTransactionHeightError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalidassetlocktransactionoutputreturnsizeerror_new = function(arg0) {
        const ret = InvalidAssetLockTransactionOutputReturnSizeError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalididentityassetlocktransactionerror_new = function(arg0) {
        const ret = InvalidIdentityAssetLockTransactionError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalididentityassetlocktransactionoutputerror_new = function(arg0) {
        const ret = InvalidIdentityAssetLockTransactionOutputError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalididentitycreditwithdrawaltransitioncorefeeerror_new = function(arg0) {
        const ret = InvalidIdentityCreditWithdrawalTransitionCoreFeeError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalididentitycreditwithdrawaltransitionoutputscripterror_new = function(arg0) {
        const ret = InvalidIdentityCreditWithdrawalTransitionOutputScriptError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_notimplementedidentitycreditwithdrawaltransitionpoolingerror_new = function(arg0) {
        const ret = NotImplementedIdentityCreditWithdrawalTransitionPoolingError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalididentitykeysignatureerror_new = function(arg0) {
        const ret = InvalidIdentityKeySignatureError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalididentitypublickeydataerror_new = function(arg0) {
        const ret = InvalidIdentityPublicKeyDataError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalididentitypublickeysecuritylevelerror_new = function(arg0) {
        const ret = InvalidIdentityPublicKeySecurityLevelError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalididentitypublickeytypeerror_new = function(arg0) {
        const ret = InvalidIdentityPublicKeyTypeError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalidinstantassetlockprooferror_new = function(arg0) {
        const ret = InvalidInstantAssetLockProofError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalidinstantassetlockproofsignatureerror_new = function(arg0) {
        const ret = InvalidInstantAssetLockProofSignatureError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_missingmasterpublickeyerror_new = function(arg0) {
        const ret = MissingMasterPublicKeyError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_missingpublickeyerror_new = function(arg0) {
        const ret = MissingPublicKeyError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_incompatibleprotocolversionerror_new = function(arg0) {
        const ret = IncompatibleProtocolVersionError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalididentifiererror_new = function(arg0) {
        const ret = InvalidIdentifierError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalidsignaturepublickeysecuritylevelerror_new = function(arg0) {
        const ret = InvalidSignaturePublicKeySecurityLevelError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalidstatetransitionsignatureerror_new = function(arg0) {
        const ret = InvalidStateTransitionSignatureError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_jsonschemacompilationerror_new = function(arg0) {
        const ret = JsonSchemaCompilationError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_jsonschemaerror_new = function(arg0) {
        const ret = JsonSchemaError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_publickeyisdisablederror_new = function(arg0) {
        const ret = PublicKeyIsDisabledError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_publickeysecuritylevelnotmeterror_new = function(arg0) {
        const ret = PublicKeySecurityLevelNotMetError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalidstatetransitiontypeerror_new = function(arg0) {
        const ret = InvalidStateTransitionTypeError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_missingstatetransitiontypeerror_new = function(arg0) {
        const ret = MissingStateTransitionTypeError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_statetransitionmaxsizeexceedederror_new = function(arg0) {
        const ret = StateTransitionMaxSizeExceededError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_unsupportedprotocolversionerror_new = function(arg0) {
        const ret = UnsupportedProtocolVersionError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_wrongpublickeypurposeerror_new = function(arg0) {
        const ret = WrongPublicKeyPurposeError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_balanceisnotenougherror_new = function(arg0) {
        const ret = BalanceIsNotEnoughError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_identitynotfounderror_new = function(arg0) {
        const ret = IdentityNotFoundError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_datacontractalreadypresenterror_new = function(arg0) {
        const ret = DataContractAlreadyPresentError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_datatriggerconditionerror_new = function(arg0) {
        const ret = DataTriggerConditionError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_datatriggerexecutionerror_new = function(arg0) {
        const ret = DataTriggerExecutionError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_datatriggerinvalidresulterror_new = function(arg0) {
        const ret = DataTriggerInvalidResultError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_documentalreadypresenterror_new = function(arg0) {
        const ret = DocumentAlreadyPresentError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_documentnotfounderror_new = function(arg0) {
        const ret = DocumentNotFoundError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_documentowneridmismatcherror_new = function(arg0) {
        const ret = DocumentOwnerIdMismatchError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_documenttimestampwindowviolationerror_new = function(arg0) {
        const ret = DocumentTimestampWindowViolationError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_documenttimestampsmismatcherror_new = function(arg0) {
        const ret = DocumentTimestampsMismatchError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_duplicateuniqueindexerror_new = function(arg0) {
        const ret = DuplicateUniqueIndexError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invaliddocumentrevisionerror_new = function(arg0) {
        const ret = InvalidDocumentRevisionError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_duplicatedidentitypublickeyidstateerror_new = function(arg0) {
        const ret = DuplicatedIdentityPublicKeyIdStateError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_duplicatedidentitypublickeystateerror_new = function(arg0) {
        const ret = DuplicatedIdentityPublicKeyStateError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_identityalreadyexistserror_new = function(arg0) {
        const ret = IdentityAlreadyExistsError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_identitypublickeydisabledatwindowviolationerror_new = function(arg0) {
        const ret = IdentityPublicKeyDisabledAtWindowViolationError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_identitypublickeyisdisablederror_new = function(arg0) {
        const ret = IdentityPublicKeyIsDisabledError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_identitypublickeyisreadonlyerror_new = function(arg0) {
        const ret = IdentityPublicKeyIsReadOnlyError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalididentitypublickeyiderror_new = function(arg0) {
        const ret = InvalidIdentityPublicKeyIdError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalididentityrevisionerror_new = function(arg0) {
        const ret = InvalidIdentityRevisionError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_maxidentitypublickeylimitreachederror_new = function(arg0) {
        const ret = MaxIdentityPublicKeyLimitReachedError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_valueerror_new = function(arg0) {
        const ret = ValueError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_compatibleprotocolversionisnotdefinederror_new = function(arg0) {
        const ret = CompatibleProtocolVersionIsNotDefinedError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_datacontractnotpresentnotconsensuserror_new = function(arg0) {
        const ret = DataContractNotPresentNotConsensusError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_platformvalueerror_new = function(arg0) {
        const ret = PlatformValueError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_identitypublickey_new = function(arg0) {
        const ret = IdentityPublicKey.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_assetlockoutputnotfounderror_new = function(arg0) {
        const ret = AssetLockOutputNotFoundError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_assetlocktransactionisnotfounderror_new = function(arg0) {
        const ret = AssetLockTransactionIsNotFoundError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalididentityerror_new = function(arg0) {
        const ret = InvalidIdentityError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_unknownassetlockprooftypeerror_new = function(arg0) {
        const ret = UnknownAssetLockProofTypeError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_chainassetlockproof_new = function(arg0) {
        const ret = ChainAssetLockProof.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_instantassetlockproof_new = function(arg0) {
        const ret = InstantAssetLockProof.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_identitycreatetransition_new = function(arg0) {
        const ret = IdentityCreateTransition.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_identitypublickeywithwitness_new = function(arg0) {
        const ret = IdentityPublicKeyWithWitness.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_identitytopuptransition_new = function(arg0) {
        const ret = IdentityTopUpTransition.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_identityupdatetransition_new = function(arg0) {
        const ret = IdentityUpdateTransition.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_invalidstatetransitionerror_new = function(arg0) {
        const ret = InvalidStateTransitionError.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_precalculatedoperation_new = function(arg0) {
        const ret = PreCalculatedOperation.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_readoperation_new = function(arg0) {
        const ret = ReadOperation.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_signatureverificationoperation_new = function(arg0) {
        const ret = SignatureVerificationOperation.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_refunds_new = function(arg0) {
        const ret = Refunds.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_statetransitionexecutioncontext_new = function(arg0) {
        const ret = StateTransitionExecutionContext.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_validationresult_new = function(arg0) {
        const ret = ValidationResult.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_validatePublicKey_bee145d16f9dc2e3 = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).validatePublicKey(getArrayU8FromWasm0(arg1, arg2));
        return ret;
    };
    imports.wbg.__wbg_verifySignature_d834cbf4122363e7 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
        const ret = getObject(arg0).verifySignature(getArrayU8FromWasm0(arg1, arg2), getArrayU8FromWasm0(arg3, arg4), getArrayU8FromWasm0(arg5, arg6));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_privateKeyToPublicKey_a8c7c0801893c959 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).privateKeyToPublicKey(getArrayU8FromWasm0(arg1, arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_sign_2e214e5975b9bc30 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        const ret = getObject(arg0).sign(getArrayU8FromWasm0(arg1, arg2), getArrayU8FromWasm0(arg3, arg4));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_from_1240938ee473cab4 = function(arg0, arg1) {
        const ret = Buffer.from(getArrayU8FromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_from_2bc38367b042f514 = function(arg0, arg1) {
        var v0 = getArrayU8FromWasm0(arg0, arg1).slice();
        wasm.__wbindgen_free(arg0, arg1 * 1);
        const ret = Buffer.from(v0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_buffer_a2f9733fd9333302 = function(arg0) {
        const ret = getObject(arg0).buffer;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_byteOffset_82ed8366a41aeb88 = function(arg0) {
        const ret = getObject(arg0).byteOffset;
        return ret;
    };
    imports.wbg.__wbg_length_ef51e3e9b7ed1303 = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_new_c161e65c73f04c8b = function(arg0) {
        const ret = new default1(takeObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_toBuffer_7d042eb39e089dc9 = function(arg0, arg1) {
        const ret = getObject(arg1).toBuffer();
        const ptr0 = passArray8ToWasm0(ret, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    };
    imports.wbg.__wbg_fetchDataContract_b17bbb6c3e7c09f9 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).fetchDataContract(takeObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_storeDataContract_dd0024afb8d1eabf = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).storeDataContract(DataContract.__wrap(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_updateDataContract_c60215d0b8f6fe2f = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).updateDataContract(DataContract.__wrap(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_createDocument_b2c811b5034750ef = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).createDocument(ExtendedDocument.__wrap(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_updateDocument_eeec09b66d85114b = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).updateDocument(ExtendedDocument.__wrap(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_removeDocument_cbc5876cf8f78808 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        try {
            const ret = getObject(arg0).removeDocument(DataContract.__wrap(arg1), getStringFromWasm0(arg2, arg3), takeObject(arg4), getObject(arg5));
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(arg2, arg3);
        }
    }, arguments) };
    imports.wbg.__wbg_fetchDocuments_0801a4db466dc34a = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        try {
            const ret = getObject(arg0).fetchDocuments(takeObject(arg1), getStringFromWasm0(arg2, arg3), takeObject(arg4), getObject(arg5));
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(arg2, arg3);
        }
    }, arguments) };
    imports.wbg.__wbg_fetchExtendedDocuments_94a7443193503e69 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        try {
            const ret = getObject(arg0).fetchExtendedDocuments(takeObject(arg1), getStringFromWasm0(arg2, arg3), takeObject(arg4), getObject(arg5));
            return addHeapObject(ret);
        } finally {
            wasm.__wbindgen_free(arg2, arg3);
        }
    }, arguments) };
    imports.wbg.__wbg_fetchIdentity_fd6d3dfa69754314 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).fetchIdentity(takeObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_createIdentity_ff3602772b6bb952 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).createIdentity(Identity.__wrap(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_addKeysToIdentity_2c4fff087a2a1cb7 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).addKeysToIdentity(takeObject(arg1), takeObject(arg2), getObject(arg3));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_disableIdentityKeys_dc6ea77b0e9a607b = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        const ret = getObject(arg0).disableIdentityKeys(takeObject(arg1), takeObject(arg2), takeObject(arg3), getObject(arg4));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_updateIdentityRevision_84e24c72e4880559 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).updateIdentityRevision(takeObject(arg1), takeObject(arg2), getObject(arg3));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_fetchIdentityBalance_3fbb7495763f58c7 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).fetchIdentityBalance(takeObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_fetchIdentityBalanceWithDebt_8dee3c90f7950264 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).fetchIdentityBalanceWithDebt(takeObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_addToIdentityBalance_2ff96a7db01dc982 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).addToIdentityBalance(takeObject(arg1), takeObject(arg2), getObject(arg3));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_addToSystemCredits_3d60d129a7018de6 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).addToSystemCredits(takeObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_fetchLatestPlatformCoreChainLockedHeight_8d163d6e82484f9e = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).fetchLatestPlatformCoreChainLockedHeight();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_fetchLatestPlatformBlockHeight_7602cbd0107bf79e = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).fetchLatestPlatformBlockHeight();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_fetchTransaction_5d234621e287b160 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).fetchTransaction(takeObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_isAssetLockTransactionOutPointAlreadyUsed_dd7675a3da46d91d = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).isAssetLockTransactionOutPointAlreadyUsed(takeObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_verifyInstantLock_040684c5609b57e5 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        var v0 = getArrayU8FromWasm0(arg1, arg2).slice();
        wasm.__wbindgen_free(arg1, arg2 * 1);
        const ret = getObject(arg0).verifyInstantLock(v0, getObject(arg3));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_markAssetLockTransactionOutPointAsUsed_60ff0f4bb3b846e5 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).markAssetLockTransactionOutPointAsUsed(takeObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_isInTheValidMasterNodesList_6699d3e331993616 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).isInTheValidMasterNodesList(takeObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_fetchLatestPlatformBlockHeader_4b787fcb3f60efa9 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).fetchLatestPlatformBlockHeader();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_fetchLatestPlatformBlockTime_1b10f5ab1c0a0ed8 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).fetchLatestPlatformBlockTime();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_generate_1f42caa3e9e2b460 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).generate();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_set_28fb264d152742af = function(arg0, arg1, arg2, arg3) {
        set(getObject(arg0), getStringFromWasm0(arg1, arg2), takeObject(arg3));
    };
    imports.wbg.__wbindgen_cb_drop = function(arg0) {
        const obj = takeObject(arg0).original;
        if (obj.cnt-- == 1) {
            obj.a = 0;
            return true;
        }
        const ret = false;
        return ret;
    };
    imports.wbg.__wbg_debug_7960d327fd96f71a = function(arg0, arg1, arg2, arg3) {
        console.debug(getObject(arg0), getObject(arg1), getObject(arg2), getObject(arg3));
    };
    imports.wbg.__wbg_error_fe807da27c4a4ced = function(arg0) {
        console.error(getObject(arg0));
    };
    imports.wbg.__wbg_error_fd84ca2a8a977774 = function(arg0, arg1, arg2, arg3) {
        console.error(getObject(arg0), getObject(arg1), getObject(arg2), getObject(arg3));
    };
    imports.wbg.__wbg_info_5566be377f5b52ae = function(arg0, arg1, arg2, arg3) {
        console.info(getObject(arg0), getObject(arg1), getObject(arg2), getObject(arg3));
    };
    imports.wbg.__wbg_log_7b690f184ae4519b = function(arg0, arg1, arg2, arg3) {
        console.log(getObject(arg0), getObject(arg1), getObject(arg2), getObject(arg3));
    };
    imports.wbg.__wbg_warn_48cbddced45e5414 = function(arg0, arg1, arg2, arg3) {
        console.warn(getObject(arg0), getObject(arg1), getObject(arg2), getObject(arg3));
    };
    imports.wbg.__wbg_String_88810dfeb4021902 = function(arg0, arg1) {
        const ret = String(getObject(arg1));
        const ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    };
    imports.wbg.__wbg_getwithrefkey_5e6d9547403deab8 = function(arg0, arg1) {
        const ret = getObject(arg0)[getObject(arg1)];
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_set_841ac57cff3d672b = function(arg0, arg1, arg2) {
        getObject(arg0)[takeObject(arg1)] = takeObject(arg2);
    };
    imports.wbg.__wbindgen_jsval_loose_eq = function(arg0, arg1) {
        const ret = getObject(arg0) == getObject(arg1);
        return ret;
    };
    imports.wbg.__wbindgen_bigint_from_i128 = function(arg0, arg1) {
        const ret = arg0 << BigInt(64) | BigInt.asUintN(64, arg1);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_bigint_from_u128 = function(arg0, arg1) {
        const ret = BigInt.asUintN(64, arg0) << BigInt(64) | BigInt.asUintN(64, arg1);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_crypto_e1d53a1d73fb10b8 = function(arg0) {
        const ret = getObject(arg0).crypto;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_msCrypto_6e7d3e1f92610cbb = function(arg0) {
        const ret = getObject(arg0).msCrypto;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_getRandomValues_805f1c3d65988a5a = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).getRandomValues(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_randomFillSync_6894564c2c334c42 = function() { return handleError(function (arg0, arg1, arg2) {
        getObject(arg0).randomFillSync(getArrayU8FromWasm0(arg1, arg2));
    }, arguments) };
    imports.wbg.__wbg_require_78a3dcfbdba9cbce = function() { return handleError(function () {
        const ret = module.require;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_process_038c26bf42b093f8 = function(arg0) {
        const ret = getObject(arg0).process;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_versions_ab37218d2f0b24a8 = function(arg0) {
        const ret = getObject(arg0).versions;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_node_080f4b19d15bc1fe = function(arg0) {
        const ret = getObject(arg0).node;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_is_string = function(arg0) {
        const ret = typeof(getObject(arg0)) === 'string';
        return ret;
    };
    imports.wbg.__wbg_new_b525de17f44a8943 = function() {
        const ret = new Array();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_get_27fe3dac1c4d0224 = function(arg0, arg1) {
        const ret = getObject(arg0)[arg1 >>> 0];
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_set_17224bc548dd1d7b = function(arg0, arg1, arg2) {
        getObject(arg0)[arg1 >>> 0] = takeObject(arg2);
    };
    imports.wbg.__wbg_from_67ca20fa722467e6 = function(arg0) {
        const ret = Array.from(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_isArray_39d28997bf6b96b4 = function(arg0) {
        const ret = Array.isArray(getObject(arg0));
        return ret;
    };
    imports.wbg.__wbg_length_e498fbc24f9c1d4f = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_push_49c286f04dd3bf59 = function(arg0, arg1) {
        const ret = getObject(arg0).push(getObject(arg1));
        return ret;
    };
    imports.wbg.__wbg_toString_11eae865ad619826 = function(arg0) {
        const ret = getObject(arg0).toString();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_instanceof_ArrayBuffer_a69f02ee4c4f5065 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof ArrayBuffer;
        } catch {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_BigInt_5584c686fbfa65cf = function() { return handleError(function (arg0) {
        const ret = BigInt(getObject(arg0));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_BigInt_a560cc1998a032e3 = function(arg0) {
        const ret = BigInt(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_toString_1359bab35813c57c = function(arg0, arg1, arg2) {
        const ret = getObject(arg1).toString(arg2);
        const ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    };
    imports.wbg.__wbg_instanceof_Error_749a7378f4439ee0 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Error;
        } catch {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_message_a95c3ef248e4b57a = function(arg0) {
        const ret = getObject(arg0).message;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_toString_cec163b212643722 = function(arg0) {
        const ret = getObject(arg0).toString();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_newwithargs_d66a68ef9c159f0d = function(arg0, arg1, arg2, arg3) {
        const ret = new Function(getStringFromWasm0(arg0, arg1), getStringFromWasm0(arg2, arg3));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_newnoargs_2b8b6bd7753c76ba = function(arg0, arg1) {
        const ret = new Function(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_call_95d1ea488d03e4e8 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).call(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_call_9495de66fdbe016b = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).call(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_name_4e66d4cfa3e9270a = function(arg0) {
        const ret = getObject(arg0).name;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_f841cc6f2098f4b5 = function() {
        const ret = new Map();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_set_388c4c6422704173 = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).set(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_next_88560ec06a094dea = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).next();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_next_b7d530c04fd8b217 = function(arg0) {
        const ret = getObject(arg0).next;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_done_1ebec03bbd919843 = function(arg0) {
        const ret = getObject(arg0).done;
        return ret;
    };
    imports.wbg.__wbg_value_6ac8da5cc5b3efda = function(arg0) {
        const ret = getObject(arg0).value;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_isSafeInteger_8c4789029e885159 = function(arg0) {
        const ret = Number.isSafeInteger(getObject(arg0));
        return ret;
    };
    imports.wbg.__wbg_getTime_7c59072d1651a3cf = function(arg0) {
        const ret = getObject(arg0).getTime();
        return ret;
    };
    imports.wbg.__wbg_new_f127e324c1313064 = function(arg0) {
        const ret = new Date(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new0_25059e40b1c02766 = function() {
        const ret = new Date();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_now_931686b195a14f9d = function() {
        const ret = Date.now();
        return ret;
    };
    imports.wbg.__wbg_setTime_ecc3c6adcf43db6c = function(arg0, arg1) {
        const ret = getObject(arg0).setTime(arg1);
        return ret;
    };
    imports.wbg.__wbg_constructor_0c9828c8a7cf1dc6 = function(arg0) {
        const ret = getObject(arg0).constructor;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_entries_4e1315b774245952 = function(arg0) {
        const ret = Object.entries(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_getPrototypeOf_bc92b90803c143ac = function(arg0) {
        const ret = Object.getPrototypeOf(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_keys_60443f4f867207f9 = function(arg0) {
        const ret = Object.keys(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_f9876326328f45ed = function() {
        const ret = new Object();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_iterator_55f114446221aa5a = function() {
        const ret = Symbol.iterator;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_9d3a9ce4282a18a8 = function(arg0, arg1) {
        try {
            var state0 = {a: arg0, b: arg1};
            var cb0 = (arg0, arg1) => {
                const a = state0.a;
                state0.a = 0;
                try {
                    return __wbg_adapter_1561(a, state0.b, arg0, arg1);
                } finally {
                    state0.a = a;
                }
            };
            const ret = new Promise(cb0);
            return addHeapObject(ret);
        } finally {
            state0.a = state0.b = 0;
        }
    };
    imports.wbg.__wbg_resolve_fd40f858d9db1a04 = function(arg0) {
        const ret = Promise.resolve(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_then_ec5db6d509eb475f = function(arg0, arg1) {
        const ret = getObject(arg0).then(getObject(arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_then_f753623316e2873a = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_globalThis_87cbb8506fecf3a9 = function() { return handleError(function () {
        const ret = globalThis.globalThis;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_self_e7c1f827057f6584 = function() { return handleError(function () {
        const ret = self.self;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_window_a09ec664e14b1b81 = function() { return handleError(function () {
        const ret = window.window;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_global_c85a9259e621f3db = function() { return handleError(function () {
        const ret = global.global;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_instanceof_Uint8Array_01cebe79ca606cca = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Uint8Array;
        } catch {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_new_537b7341ce90bb31 = function(arg0) {
        const ret = new Uint8Array(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_newwithlength_b56c882b57805732 = function(arg0) {
        const ret = new Uint8Array(arg0 >>> 0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_newwithbyteoffsetandlength_9fb2f11355ecadf5 = function(arg0, arg1, arg2) {
        const ret = new Uint8Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_subarray_7526649b91a252a6 = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).subarray(arg1 >>> 0, arg2 >>> 0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_length_27a2afe8ab42b09f = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_set_17499e8aa4003ebd = function(arg0, arg1, arg2) {
        getObject(arg0).set(getObject(arg1), arg2 >>> 0);
    };
    imports.wbg.__wbg_get_baf4855f9a986186 = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.get(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_has_3feea89d34bd7ad5 = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.has(getObject(arg0), getObject(arg1));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_set_6aa458a4ebdb65cb = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = Reflect.set(getObject(arg0), getObject(arg1), getObject(arg2));
        return ret;
    }, arguments) };
    imports.wbg.__wbindgen_is_function = function(arg0) {
        const ret = typeof(getObject(arg0)) === 'function';
        return ret;
    };
    imports.wbg.__wbindgen_object_drop_ref = function(arg0) {
        takeObject(arg0);
    };
    imports.wbg.__wbg_parse_3ac95b51fc312db8 = function() { return handleError(function (arg0, arg1) {
        const ret = JSON.parse(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_stringify_87eff7b5028c80b6 = function() { return handleError(function (arg0, arg1) {
        const ret = JSON.stringify(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_buffer_cf65c07de34b9a08 = function(arg0) {
        const ret = getObject(arg0).buffer;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_debug_string = function(arg0, arg1) {
        const ret = debugString(getObject(arg1));
        const ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    };
    imports.wbg.__wbindgen_bigint_get_as_i64 = function(arg0, arg1) {
        const v = getObject(arg1);
        const ret = typeof(v) === 'bigint' ? v : undefined;
        getBigInt64Memory0()[arg0 / 8 + 1] = isLikeNone(ret) ? BigInt(0) : ret;
        getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
    };
    imports.wbg.__wbindgen_throw = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
    };
    imports.wbg.__wbindgen_memory = function() {
        const ret = wasm.memory;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper23086 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 725, __wbg_adapter_58);
        return addHeapObject(ret);
    };

    return imports;
}

function initMemory(imports, maybe_memory) {

}

function finalizeInit(instance, module) {
    wasm = instance.exports;
    init.__wbindgen_wasm_module = module;
    cachedBigInt64Memory0 = null;
    cachedFloat64Memory0 = null;
    cachedInt32Memory0 = null;
    cachedUint32Memory0 = null;
    cachedUint8Memory0 = null;


    return wasm;
}

function initSync(module) {
    const imports = getImports();

    initMemory(imports);

    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }

    const instance = new WebAssembly.Instance(module, imports);

    return finalizeInit(instance, module);
}

async function init(input) {

    const imports = getImports();

    if (typeof input === 'string' || (typeof Request === 'function' && input instanceof Request) || (typeof URL === 'function' && input instanceof URL)) {
        input = fetch(input);
    }

    initMemory(imports);

    const { instance, module } = await load(await input, imports);

    return finalizeInit(instance, module);
}

export { initSync }
export default init;

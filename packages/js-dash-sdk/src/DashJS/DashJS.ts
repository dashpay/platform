import {SDK as _SDK} from './SDK';
import {
    Transaction as _Transaction,
    Address as _Address,
    Block as _Block,
    UnspentOutput as _UnspentOutput,
    HDPublicKey as _HDPublicKey,
    HDPrivateKey as _HDPrivateKey,
    Mnemonic as _Mnemonic,
    Network as _Network,
    Input as _Input,
    Output as _Output,
    Script as _Script,
    PublicKey as _PublicKey,
    PrivateKey as _PrivateKey
} from '@dashevo/dashcore-lib';

export namespace DashJS {
    export let SDK = _SDK;

    // Dashcore-lib primitives
    export let Transaction = _Transaction;

    export let Address = _Address;
    export let Block = _Block;
    export let UnspentOutput = _UnspentOutput;
    export let HDPublicKey = _HDPublicKey;
    export let HDPrivateKey = _HDPrivateKey;
    export let PublicKey = _PublicKey;
    export let PrivateKey = _PrivateKey;
    export let Mnemonic = _Mnemonic;
    export let Network = _Network;
    export let Script = _Script;
    export let Input = _Input;
    export let Output = _Output;
}


export {DashJS as default};

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

export declare namespace Core {
    export type Transaction = _Transaction;

    export type Address = _Address;
    export type Block = _Block;
    export type UnspentOutput = _UnspentOutput;
    export type HDPublicKey = _HDPublicKey;
    export type HDPrivateKey = _HDPrivateKey;
    export type PublicKey = _PublicKey;
    export type PrivateKey = _PrivateKey;
    export type Mnemonic = _Mnemonic;
    export type Network = _Network;
    export type Script = _Script;
    export type Input = _Input;
    export type Output = _Output;
}

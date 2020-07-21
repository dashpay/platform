import {PrivateKey, Network,} from "../types";
import {HDPrivateKey, HDPublicKey} from "@dashevo/dashcore-lib";
import {Transaction} from "@dashevo/dashcore-lib/typings/transaction/Transaction";

export declare namespace KeyChain {
    interface IKeyChainOptions {
        network?: Network;
        keys?: [Keys]
    }
}

export declare class KeyChain {
    constructor(options?: KeyChain.IKeyChainOptions);
    network: Network;
    keys: [Keys];

    type: HDKeyTypesParam|PrivateKeyTypeParam;
    HDPrivateKey?: HDPrivateKey;
    privateKey?: PrivateKey;

    generateKeyForChild(index: number, type?: HDKeyTypesParam): HDPrivateKey|HDPublicKey;
    generateKeyForPath(path: string, type?: HDKeyTypesParam): HDPrivateKey|HDPublicKey;

    getHardenedBIP44Path(type?: HDKeyTypesParam): HDKeyTypes;
    getHardenedDIP9FeaturePath(type?: HDKeyTypesParam): HDKeyTypes;

    getKeyForChild(index: number, type?: HDKeyTypesParam): HDKeyTypes;
    getKeyForPath(path: string, type?: HDKeyTypesParam): HDKeyTypes;
    getPrivateKey(): PrivateKey;

    sign(object: Transaction|any, privateKeys:[PrivateKey], sigType: number): any;
}

type HDKeyTypes = HDPublicKey | HDPrivateKey;

export declare enum HDKeyTypesParam {
    HDPrivateKey="HDPrivateKey",
    HDPublicKey="HDPrivateKey",
}
export declare enum PrivateKeyTypeParam {
    privateKey='privateKey'
}
export declare interface Keys {
    [path: string]: {
        path: string
    };
}



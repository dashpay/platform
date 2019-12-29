import {PrivateKey, Network,} from "../types";
import {HDPrivateKey, HDPublicKey} from "@dashevo/dashcore-lib";

export declare class KeyChain {
    constructor(options?: KeyChain.Options);
    network: Network;
    keys: [Keys];

    // todo valid keychain type object.
    type: string;
    HDPrivateKey?: HDPrivateKey;
    privateKey?: PrivateKey;


    generateKeyForChild(index: number, type?: HDKeyTypesParam): HDPrivateKey|HDPublicKey;
    generateKeyForPath(path: string, type?: HDKeyTypesParam): HDKeyTypes;

    getHardenedBIP44Path(type?: HDKeyTypesParam): HDKeyTypes;
    getHardenedDIP9FeaturePath(type?: HDKeyTypesParam): HDKeyTypes;

    getKeyForChild(index: number, type?: HDKeyTypesParam): HDKeyTypes;
    getKeyForPath(path: string, type?: HDKeyTypesParam): HDKeyTypes;
    getPrivateKey(): HDPrivateKey|PrivateKey;

    // TODO : dashcore-lib miss an implementation definition of crypto.Signature
    sign(object: any, privateKeys:[any], sigType: object): any;

}
type HDKeyTypes = HDPublicKey | HDPrivateKey;

export declare enum HDKeyTypesParam {
    HDPrivateKey="HDPrivateKey",
    HDPublicKey="HDPrivateKey",
}
export declare interface Keys {
    [path: string]: {
        path: string
    };
}
export declare namespace KeyChain {
    interface Options {
        network?: Network;
        keys?: [Keys]
    }
}



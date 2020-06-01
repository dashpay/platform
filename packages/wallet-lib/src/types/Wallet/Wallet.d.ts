import {Mnemonic, PrivateKey, HDPublicKey, Strategy, Network, Plugins, AddressInfoMap, WalletType} from "../types";
import {Account} from "../Account/Account";
import {MappedAddress} from "../Storage/Storage";
import {HDPrivateKey} from "@dashevo/dashcore-lib";

export declare class Wallet {
    offlineMode: boolean;
    allowSensitiveOperations: boolean;
    injectDefaultPlugins: boolean;
    plugins:[Plugins];
    passphrase?:string;
    constructor(options?: Wallet.Options);
    createAccount(accOptions: Account.Options): Promise<Account>;
    disconnect(): void;
    exportWallet():Mnemonic["toString"];
    fromMnemonic(mnemonic: Mnemonic):void;
    fromPrivateKey(privateKey: PrivateKey):void;
    fromHDPrivateKey(privateKey: HDPrivateKey):void;
    fromHDPublicKey(HDPublicKey:HDPublicKey):void;
    fromSeed(seed:string):void;
    generateNewWalletId():void;
    getAccount(accOptions?: Wallet.getAccOptions): Promise<Account>;
    updateNetwork(network:Network):boolean;

}

export declare namespace Wallet {

    interface Options {
        debug?: boolean;
        offlineMode?: boolean;
        transporter?: string|object|any;
        network?: Network;
        plugins?: [Plugins];
        passphrase?: string;
        injectDefaultPlugins?: boolean;
        mnemonic?: Mnemonic|string|null;
        seed?: Mnemonic|string;
        privateKey?: PrivateKey|string;
        HDPrivateKey?: HDPrivateKey|string;
        HDPublicKey?: HDPublicKey|string;
    }
    interface getAccOptions extends Account.Options{
        index?:number;
    }
}



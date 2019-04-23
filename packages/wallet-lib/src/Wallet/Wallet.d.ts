import {Mnemonic,PrivateKey, HDPublicKey, Strategy, Network, Plugins} from "../types";
import {Account} from "../Account/Account";

export declare class Wallet {
    constructor(options?: Wallet.Options);

    createAccount(accOptions: Account.Options): Account;
    disconnect(): void;
    exportWallet():Mnemonic["toString"];
    fromMnemonic(Mnemonic):void;
    fromPrivateKey(PrivateKey):void;
    fromHDPubKey(HDPublicKey):void;
    fromSeed(seed):void;
    generateNewWalletId():void;
    getAccount(accOptions: Wallet.getAccOptions): Account;
    updateNetwork(Network):boolean;

}
export declare namespace Wallet {
    interface Options {
        offlineMode?: boolean;
        network?: Network;
        plugins?: [Plugins];
        passphrase?: string;
        injectDefaultPlugins?: boolean;
        mnemonic?: Mnemonic
        seed?: Mnemonic
    }
    interface getAccOptions extends Account.Options{
        index?:number;
    }
}


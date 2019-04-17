export declare type transactionId<T extends string = string> = T;
export declare type Mnemonic<T extends object = object> = T & {
    toString(): string;
};
export declare type PrivateKey<T extends object = object> = T & {
    toString(): string;
};
export declare type PublicKey<T extends object = object> = T & {
    toString(): string;
};
export declare type Seed<T extends object = object> = T & {
    toString(): string;
};
export declare type Transaction<T extends object = object> = T & {
    toString(): string;
};
export declare type TransactionInfo<T extends object = object> = T & {
    txid:string;
    blockhash:string;
    blockheight:number
    blocktime: string
    fees: number;
    size:number;
    vout:[object];
    vin:[object];
    txlock:boolean;
};
export declare type Plugins<T extends object = object> = T & {
    toString(): string;
};
export declare type PublicAddress<T extends string = string> = T;
export declare type Address<T extends object = object> = T & {
    toString(): string;
};
export declare type AddressObj<T extends object = object> = T & {
    address: string;
    path: string;
}
export declare type AddressInfo<T extends AddressObj = AddressObj> = T & {
    balanceSat: number;
    fetchedLast:number;
    unconfirmedBalanceSat: number;
    transaction: object;
    used:boolean;
    utxos:[object]
}
export declare type Network = "livenet" | "testnet";
export declare type Strategy = "livenet" | "testnet";
export declare type AddressType = "external" | "internal" | "misc";

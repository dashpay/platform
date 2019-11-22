// import {Wallet} from "@dashevo/wallet-lib";
import {Wallet} from "../../../../wallet-lib/src";
import {Mnemonic, Network} from "@dashevo/wallet-lib/src/types";
import {Platform, PlatformOpts} from './Platform';
// @ts-ignore
import DAPIClient from "@dashevo/dapi-client"
import {Schema} from "inspector";

const defaultSeeds = [
    '18.237.69.61',
    '18.236.234.255',
].map(ip => ({service: `${ip}:3000`}));


export type DPASchema = object

export interface SDKOpts {
    network?: Network;
    mnemonic?: Mnemonic | string,
    schemas?:SDKSchemas;
}

export type SDKClient = object | DAPIClient;

export interface SDKClients {
    [name: string]: SDKClient,
    dapi: DAPIClient
}
export interface SDKSchemas {
    [name:string]: DPASchema
}

export class SDK {
    public network: string = 'testnet';
    public wallet: Wallet | undefined;
    public platform: Platform | undefined;
    private readonly clients: SDKClients;
    private readonly schemas: SDKOpts['schemas'];

    constructor(opts: SDKOpts) {
        this.network = opts && opts.network || 'testnet';
        this.schemas = opts && opts.schemas;
        this.clients = {
            dapi: new DAPIClient(Object.assign({
                seeds: defaultSeeds,
                timeout: 20000,
                retries: 15
            }, opts || {network: this.network}))
        }
        if(opts.mnemonic){
            // @ts-ignore
            this.wallet = new Wallet({...opts, offlineMode: !(opts && opts.mnemonic)});
        }
        if(opts.schemas!== undefined){
            let platformOpts: PlatformOpts = {
                client: this.getDAPIInstance(),
                schemas : this.getSchemas()
            };
            this.platform = new Platform(platformOpts)
        }
    }

    getDAPIInstance(){
        if (!!this.clients['dapi']) {
            throw new Error(`There is no client DAPI`);
        }
        return this.clients['dapi'];
    }
    addSchema(schemaName: string, schemaData: object){
        if(this.clients[schemaName]){
            throw new Error(`There is already a schema named ${schemaName}`);
        }
        this.clients[schemaName] = schemaData
    }
    getSchemas():SDKSchemas{
        if(!this.schemas){
            throw new Error(`No schemas set`);
        }
        return this.schemas;
    }
}

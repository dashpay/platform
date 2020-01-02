import {Wallet, Account} from "@dashevo/wallet-lib";
// FIXME: use dashcorelib types
import {Platform, PlatformOpts} from './Platform';
// @ts-ignore
import DAPIClient from "@dashevo/dapi-client"
import {Network, Mnemonic} from "@dashevo/dashcore-lib";

const defaultSeeds = [
    '52.26.165.185',
    '54.202.56.123',
    '54.245.133.124',
].map(ip => ({service: `${ip}:3000`}));


export type DPASchema = object

export interface SDKOpts {
    network?: Network | string,
    mnemonic?: Mnemonic | string | null,
    apps?: SDKApps,
    accountIndex?: number,
}

export type SDKClient = object | DAPIClient;

export interface SDKClients {
    [name: string]: SDKClient,

    dapi: DAPIClient
}

export interface SDKApps {
    [name: string]: {
        contractId: string,
        contract: DPASchema
    }
}

export class SDK {
    public network: string = 'testnet';
    public wallet: Wallet | undefined;
    public account: Account | undefined;
    public platform: Platform | undefined;
    public accountIndex: number = 0;
    private readonly clients: SDKClients;
    private readonly apps: SDKApps;
    public state: { isReady: boolean, isAccountReady: boolean };

    constructor(opts: SDKOpts = {}) {

        this.network = (opts.network !== undefined) ? opts.network.toString() : 'testnet';
        this.apps = Object.assign({
            dpns: {
                contractId: '2KfMcMxktKimJxAZUeZwYkFUsEcAZhDKEpQs8GMnpUse'
            }
        }, opts.apps);

        this.state = {
            isReady: false,
            isAccountReady: false
        };
        this.clients = {
            dapi: new DAPIClient(Object.assign({
                seeds: defaultSeeds,
                timeout: 20000,
                retries: 15
            }, opts || {network: this.network}))
        }
        // We accept null as parameter for a new generated mnemonic
        if (opts.mnemonic !== undefined) {
            // @ts-ignore
            this.wallet = new Wallet({...opts, transport: this.clients.dapi});
            if (this.wallet) {
                let accountIndex = (opts.accountIndex !== undefined) ? opts.accountIndex : 0;
                this.account = this.wallet.getAccount({index: accountIndex});
            }
        }

        let platformOpts: PlatformOpts = {
            client: this.getDAPIInstance(),
            apps: this.getApps()
        };
        const self = this;
        if (this.account) {
            this.account
                .isReady()
                .then(() => {
                    // @ts-ignore
                    self.state.isAccountReady = true;
                })
        } else {
            // @ts-ignore
            this.state.isAccountReady = true;
        }
        this.platform = new Platform({
            ...platformOpts,
            network: this.network,
            account: this.account,
        })

        const promises = [];
        for (let appName in this.apps) {
            const app = this.apps[appName];
            try {
                const p = this.platform?.contracts.get(app.contractId);
                promises.push(p);
            } catch (e) {
                console.error(e);
            }
        }
        Promise
            .all(promises)
            .then((res) => {this.state.isReady = true});
    }

    async isReady() {
        const self = this;
        // eslint-disable-next-line consistent-return
        return new Promise(((resolve) => {
            // @ts-ignore
            if (self.state.isAccountReady && self.state.isReady) return resolve(true);

            const promises = [];

            if(!self.state.isAccountReady){
                // @ts-ignore
                promises.push(self.account.isReady());
            }
            if(!self.state.isReady){
                const p = new Promise((res)=>{
                    let isReadyInterval = setInterval(() => {
                        if (self.state.isReady) {
                            clearInterval(isReadyInterval);
                            res(true);
                        }
                    }, 100);
                })
                promises.push(p);
            }

            Promise.all(promises).then((promisesResults)=>{
                resolve(true);
            });
        }));
    }

   async disconnect(){
        if(this.wallet){
            await this.wallet.disconnect();
        }
    }


    getDAPIInstance() {
        if (this.clients['dapi'] == undefined) {
            throw new Error(`There is no client DAPI`);
        }
        return this.clients['dapi'];
    }


    getApps(): SDKApps {
        return this.apps;
    }
}

export default SDK;

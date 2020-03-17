import {Wallet, Account} from "@dashevo/wallet-lib";
// FIXME: use dashcorelib types
import {Platform, PlatformOpts} from './Platform';
// @ts-ignore
import DAPIClient from "@dashevo/dapi-client"
import {Network, Mnemonic} from "@dashevo/dashcore-lib";
import isReady from "./methods/isReady";

const defaultSeeds = [
    '52.26.165.185',
    '54.202.56.123',
    '54.245.133.124',
].map(ip => ({service: `${ip}:3000`}));


export type DPASchema = object

export interface SDKOpts {
    seeds?: [string];
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
    public isReady: Function;

    constructor(opts: SDKOpts = {}) {
        this.isReady = isReady.bind(this);

        this.network = (opts.network !== undefined) ? opts.network.toString() : 'testnet';
        this.apps = Object.assign({
            dpns: {
                contractId: '77w8Xqn25HwJhjodrHW133aXhjuTsTv9ozQaYpSHACE3'
            }
        }, opts.apps);

        this.state = {
            isReady: false,
            isAccountReady: false
        };
        const seeds = (opts.seeds) ? opts.seeds : defaultSeeds;

        this.clients = {
            dapi: new DAPIClient({
                seeds: seeds,
                timeout: 1000,
                retries: 5,
                network: this.network
            })
        };
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
            const p = this.platform?.contracts.get(app.contractId);
            promises.push(p);
        }
        Promise
            .all(promises)
            .then((res) => {
                this.state.isReady = true
            })
            .catch((e) => {
                console.error('SDK apps fetching : failed to init', e);
            });

    }


    async disconnect() {
        if (this.wallet) {
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

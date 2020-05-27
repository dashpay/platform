import {Wallet, Account} from "@dashevo/wallet-lib";
// FIXME: use dashcorelib types
import {Platform, PlatformOpts} from './Platform';
// @ts-ignore
import DAPIClient from "@dashevo/dapi-client"
import {Network, Mnemonic} from "@dashevo/dashcore-lib";
import isReady from "./methods/isReady";

/**
 * default seed passed to SDK options
 */
const defaultSeeds = [
    '52.24.198.145',
    '52.13.92.167',
    '34.212.245.91',
].map(ip => ({service: `${ip}:3000`}));


export type DPASchema = object

/**
 * Interface Client Options
 *
 * @param {[string]?} [seeds] - wallet seeds
 * @param {Network? | string?} [network] - evonet network
 * @param {Wallet.Options? | null?} [wallet] - wallet options
 * @param {SDKApps?} [apps] - applications
 * @param {number?} [accountIndex] - account index number
 */
export interface ClientOpts {
    seeds?: [string];
    network?: Network | string,
    wallet?: Wallet.Options | null,
    apps?: ClientApps,
    accountIndex?: number,
}

/**
 * Defined Type for ClientDependency
 */
export type ClientDependency = DAPIClient | any;

/**
 * Interface for ClientDependencies
 * @typeparam ClientDependencies object or DAPIClient
 */
export interface ClientDependencies {
    [name: string]: ClientDependency,
}

/**
 * Interface for ClientApps
 */
export interface ClientApps {
    [name: string]: {
        contractId: string,
        contract: DPASchema
    }
}

/**
 * class for SDK
 */
export class Client {
    public network: string = 'testnet';
    public wallet: Wallet | undefined;
    public account: Account | undefined;
    public platform: Platform | undefined;
    public accountIndex: number = 0;
    private readonly clients: ClientDependencies;
    private readonly apps: ClientApps;
    public state: { isReady: boolean, isAccountReady: boolean };
    public isReady: Function;

    /**
     * Construct some instance of SDK Client
     *
     * @param {opts} ClientOpts - options for SDK Client
     */
    constructor(opts: ClientOpts = {}) {
        this.isReady = isReady.bind(this);

        this.network = (opts.network !== undefined) ? opts.network.toString() : 'testnet';
        this.apps = Object.assign({
            dpns: {
                contractId: '295xRRRMGYyAruG39XdAibaU9jMAzxhknkkAxFE7uVkW'
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
        if (opts.wallet !== undefined) {
            // @ts-ignore
            this.wallet = new Wallet({
                transporter: {
                    seeds: seeds,
                    timeout: 1000,
                    retries: 5,
                    network: this.network,
                    type: 'dapi',
                },
                ...opts.wallet,
            });
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
            // @ts-ignore
            promises.push(p);
        }
        Promise
            .all(promises)
            .then((res) => {
                this.state.isReady = true
            })
            .catch((e) => {
                console.error('SDK apps fetching : failed to init', e);
                throw e;
            });

    }

    /**
     * disconnect wallet from Dapi
     */
    async disconnect() {
        if (this.wallet) {
            await this.wallet.disconnect();
        }
    }

    /**
     * fetch some instance of DAPI client
     *
     * @remarks
     * This function throws an error message when there is no client DAPI instance
     *
     * @returns DAPI client instance
     */
    getDAPIInstance() {
        if (this.clients['dapi'] == undefined) {
            throw new Error(`There is no client DAPI`);
        }
        return this.clients['dapi'];
    }

    /**
     * fetch list of applications
     *
     * @remarks
     * check if returned value can be null on devnet
     *
     * @returns applications list
     */
    getApps(): ClientApps {
        return this.apps;
    }
}

export default Client;

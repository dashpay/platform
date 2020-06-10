import { Account, Wallet } from "@dashevo/wallet-lib";
import { DAPIClient as DAPIClientWrapper } from "@dashevo/wallet-lib/src/transporters"
// FIXME: use dashcorelib types
import { Platform } from './Platform';
// @ts-ignore
import { Network } from "@dashevo/dashcore-lib";
import DAPIClient from "@dashevo/dapi-client";

/**
 * default seed passed to SDK options
 */
const defaultSeeds = [
    {service: 'seed-1.evonet.networks.dash.org'},
    {service: 'seed-2.evonet.networks.dash.org'},
    {service: 'seed-3.evonet.networks.dash.org'},
    {service: 'seed-4.evonet.networks.dash.org'},
    {service: 'seed-5.evonet.networks.dash.org'},
];


/**
 * Interface for DAPIClientSeed
 * @param {string} service - service seed, can be an IP, HTTP or DNS Seed
 */
export interface DAPIClientSeed {
    service: string,
}

/**
 * Interface Client Options
 *
 * @param {[string]?} [seeds] - DAPI seeds
 * @param {Network? | string?} [network] - evonet network
 * @param {Wallet.Options} [wallet] - Wallet options
 * @param {ClientApps?} [apps] - applications
 * @param {number} [walletAccountIndex=0] - account index number
 */
export interface ClientOpts {
    seeds?: DAPIClientSeed[];
    network?: Network | string,
    wallet?: Wallet.Options | null,
    apps?: ClientApps,
    walletAccountIndex?: number,
}

/**
 * Interface for ClientApps
 */
export interface ClientApps {
    [name: string]: {
        contractId: string,
        contract?: any
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
    public walletAccountIndex: number = 0;
    private readonly dapiClientWrapper: DAPIClientWrapper;
    private readonly apps: ClientApps;
    private options: ClientOpts;

    /**
     * Construct some instance of SDK Client
     *
     * @param {ClientOpts} [options] - options for SDK Client
     */
    constructor(options: ClientOpts = {}) {
        this.options = {
            walletAccountIndex: 0,
            ...options
        }

        // @ts-ignore
        this.walletAccountIndex = this.options.walletAccountIndex;

        this.network = this.options.network ? this.options.network.toString() : 'testnet';

        this.apps = Object.assign({
            dpns: {
                contractId: '7PBvxeGpj7SsWfvDSa31uqEMt58LAiJww7zNcVRP1uEM'
            }
        }, this.options.apps);

        this.dapiClientWrapper = new DAPIClientWrapper({
            seeds: this.options.seeds || defaultSeeds,
            timeout: 1000,
            retries: 5,
            network: this.network
        });

        // We accept null as parameter for a new generated mnemonic
        if (this.options.wallet !== undefined) {
            this.wallet = new Wallet({
                transporter: this.dapiClientWrapper,
                ...this.options.wallet,
            });
        }

        this.platform = new Platform({
            client: this,
            apps: this.getApps(),
            network: this.network,
        });
    }

    /**
     * Get Wallet account
     *
     * @param {Wallet.getAccOptions} [options]
     * @returns {Promise<Account>}
     */
    async getWalletAccount(options: Wallet.getAccOptions = {}) : Promise<Account> {
        if (!this.wallet) {
            throw new Error('Wallet is not initialized, pass `wallet` option to Client');
        }

        options = {
            index: this.walletAccountIndex,
            ...options,
        }

        return this.wallet.getAccount(options);
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
     * @returns {DAPIClient}
     */
    getDAPIClient() : DAPIClient {
        return this.dapiClientWrapper.client;
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

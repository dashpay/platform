import { Account, Wallet } from "@dashevo/wallet-lib";
import DAPIClientTransport from "@dashevo/wallet-lib/src/transport/DAPIClientTransport/DAPIClientTransport"
// FIXME: use dashcorelib types
import { Platform } from './Platform';
// @ts-ignore
import { Network } from "@dashevo/dashcore-lib";
import DAPIClient from "@dashevo/dapi-client";

/**
 * Interface Client Options
 *
 * @param {ClientApps?} [apps] - applications
 * @param {Wallet.Options} [wallet] - Wallet options
 * @param {number} [walletAccountIndex=0] - Wallet account index number
 * @param {DAPIAddressProvider} [dapiAddressProvider] - DAPI Address Provider instance
 * @param {Array<RawDAPIAddress|DAPIAddress|string>} [addresses] - DAPI addresses
 * @param {string[]|RawDAPIAddress[]} [seeds] - DAPI seeds
 * @param {string|Network} [network=evonet] - Network name
 * @param {number} [timeout=2000]
 * @param {number} [retries=3]
 * @param {number} [baseBanTime=60000]
 */
export interface ClientOpts {
    apps?: ClientApps,
    wallet?: Wallet.Options,
    walletAccountIndex?: number,
    dapiAddressProvider?: any,
    addresses?: any[],
    seeds?: any[],
    network?: Network | string,
    timeout?: number,
    retries?: number,
    baseBanTime?: number,
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
    public network: string = 'evonet';
    public wallet: Wallet | undefined;
    public account: Account | undefined;
    public platform: Platform | undefined;
    public walletAccountIndex: number = 0;
    private readonly dapiClient: DAPIClient;
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

        this.network = this.options.network ? this.options.network.toString() : 'evonet';

        // Initialize DAPI Client
        const dapiClientOptions = {
            network: this.network,
        };

        [
            'dapiAddressProvider',
            'addresses',
            'seeds',
            'timeout',
            'retries',
            'baseBanTime'
        ].forEach((optionName) => {
            if (this.options.hasOwnProperty(optionName)) {
                dapiClientOptions[optionName] = this.options[optionName];
            }
        });

        this.dapiClient = new DAPIClient(dapiClientOptions);

        // Initialize a wallet if `wallet` option is preset
        if (this.options.wallet !== undefined) {
            if (this.options.wallet.network !== undefined && this.options.wallet.network !== this.network) {
                throw new Error('Wallet and Client networks are different');
            }

            const transport = new DAPIClientTransport(this.dapiClient);

            this.wallet = new Wallet({
                transport,
                network: this.network,
                ...this.options.wallet,
            });

            // @ts-ignore
            this.walletAccountIndex = this.options.walletAccountIndex;
        }

        this.apps = Object.assign({
            dpns: {
                contractId: '7DVe2cDyZMf8sDjQ46XqDzbeGKncrmkD6L96QohLmLbg'
            }
        }, this.options.apps);

        this.platform = new Platform({
            client: this,
            apps: this.getApps(),
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
     * Get DAPI Client instance
     *
     * @returns {DAPIClient}
     */
    getDAPIClient() : DAPIClient {
        return this.dapiClient;
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

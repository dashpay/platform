import { EventEmitter } from 'events';
import { Account, Wallet } from "@dashevo/wallet-lib";
import DAPIClientTransport from "@dashevo/wallet-lib/src/transport/DAPIClientTransport/DAPIClientTransport"
import { Platform } from './Platform';
import { Network } from "@dashevo/dashcore-lib";
import DAPIClient from "@dashevo/dapi-client";
import { ClientApps, ClientAppsOptions } from "./ClientApps";

export interface WalletOptions extends Wallet.IWalletOptions {
    defaultAccountIndex?: number;
}

/**
 * Interface Client Options
 *
 * @param {ClientApps?} [apps] - applications
 * @param {WalletOptions} [wallet] - Wallet options
 * @param {DAPIAddressProvider} [dapiAddressProvider] - DAPI Address Provider instance
 * @param {Array<RawDAPIAddress|DAPIAddress|string>} [dapiAddresses] - DAPI addresses
 * @param {string[]|RawDAPIAddress[]} [seeds] - DAPI seeds
 * @param {string|Network} [network=evonet] - Network name
 * @param {number} [timeout=2000]
 * @param {number} [retries=3]
 * @param {number} [baseBanTime=60000]
 */
export interface ClientOpts {
    apps?: ClientAppsOptions,
    wallet?: WalletOptions,
    dapiAddressProvider?: any,
    dapiAddresses?: any[],
    seeds?: any[],
    network?: Network | string,
    timeout?: number,
    retries?: number,
    baseBanTime?: number,
}

/**
 * Client class that wraps all components together to allow integrated payments on both the Dash Network (layer 1)
 * and the Dash Platform (layer 2).
 */
export class Client extends EventEmitter {
    public network: string = 'testnet';
    public wallet: Wallet | undefined;
    public account: Account | undefined;
    public platform: Platform;
    public defaultAccountIndex: number | undefined = 0;
    private readonly dapiClient: DAPIClient;
    private readonly apps: ClientApps;
    private options: ClientOpts;

    /**
     * Construct some instance of SDK Client
     *
     * @param {ClientOpts} [options] - options for SDK Client
     */
    constructor(options: ClientOpts = {}) {
        super();

        this.options = options;

        this.network = this.options.network ? this.options.network.toString() : 'testnet';

        // Initialize DAPI Client
        const dapiClientOptions = {
            network: this.network,
        };

        [
            'dapiAddressProvider',
            'dapiAddresses',
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
            this.wallet.on('error', (error, context) => (
                this.emit('error', error, { wallet: context })
            ));
        }

        // @ts-ignore
        this.defaultAccountIndex = this.options.wallet?.defaultAccountIndex || 0;

        this.apps = new ClientApps(Object.assign({
            dpns: {
                contractId: '76wgB8KBxLGhtEzn4Hp5zgheyzzpHYvfcWGLs69B2ahq'
            },
            dashpay: {
                contractId: '6wfobip5Mfn6NNGK9JTQ5eHtZozpkNx4aZUsnCxkfgj5',
            },
        }, this.options.apps));

        this.platform = new Platform({
            client: this,
        });
    }

    /**
     * Get Wallet account
     *
     * @param {Account.Options} [options]
     * @returns {Promise<Account>}
     */
    async getWalletAccount(options: Account.Options = {}) : Promise<Account> {
        if (!this.wallet) {
            throw new Error('Wallet is not initialized, pass `wallet` option to Client');
        }

        options = {
            index: this.defaultAccountIndex,
            ...options,
        }

        return this.wallet.getAccount(options);
    }

    /**
     * disconnect wallet from Dapi
     * @returns {void}
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
     * @returns {ClientApps} applications list
     */
    getApps(): ClientApps {
        return this.apps;
    }
}

export default Client;

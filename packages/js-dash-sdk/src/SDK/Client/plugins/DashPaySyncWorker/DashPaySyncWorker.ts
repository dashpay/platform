import { plugins} from "@dashevo/wallet-lib"

export class DashPaySyncWorker extends plugins.Worker {
  private fromTimestamp: number;
  private platform?: any;
  private walletId: any;
  private identities: any;
  private getPlugin: any;
  private storage: any;
  private contacts: any[];
  private keyChainStore: any;

  constructor() {
    super({
      name: 'DashPaySyncWorker',
      executeOnStart: true,
      firstExecutionRequired: true,
      awaitOnInjection: true,
      workerIntervalTime: 60 * 1000,
      dependencies: [
        'storage',
        'keyChainStore',
        'getWorker',
        'getPlugin',
        'walletId',
        'identities',
        'getUnusedIdentityIndex',
      ],
      injectionOrder: {
        after: [
          'IdentitySyncWorker',
          'DashPay'
        ]
      }
    });
    this.contacts = [];
    this.fromTimestamp = 0;
  }

  async onStart(){

  }

  async execute() {
    if (this.platform && this.platform.identities) {
      const dashPay = await this.getPlugin('DashPay');
      const walletStore = this.storage.getWalletStore(this.walletId)
      const identities = walletStore.getIndexedIdentityIds(this.walletId);

      // We require an identity to fetch contacts
      if (identities.length) {
        const contacts = await dashPay.fetchEstablishedContacts(this.fromTimestamp);
        // set 10 minute before last query
        // see: https://github.com/dashpay/dips/blob/master/dip-0015.md#fetching-contact-requests
        this.fromTimestamp = +new Date() - 10 * 60 * 1000;
        // const walletStore = this.storage.getWalletStore(this.walletId);
        // const addressesStore = this.storage.store.wallets[this.walletId].addresses;
        //@ts-ignore
        const txStreamSyncWorker = await this.getWorker('TransactionSyncStreamWorker');

        contacts
          .forEach((contact) => {
            console.log(`DashPaySyncWorker - Fetched contact ${contact.username}`);
            this.contacts.push(contact);
            dashPay.contacts.push(contact);

            this.keyChainStore.addKeyChain(contact.keychains.receiving);
            this.keyChainStore.addKeyChain(contact.keychains.sending);
            //@ts-ignore
          })

        await txStreamSyncWorker.onStop()
        await txStreamSyncWorker.onStart();
      }
    }
  }

  async onStop() {
  }
}

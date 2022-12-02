/* eslint-disable no-console */
const logger = require('../src/logger');
const Dash = require('../../js-dash-sdk');

 let baseClientOpts = {
  dapiAddresses: [
    // IP or url(s) pointing to your server(s)
    'crux:3000:3010',
    'crux:3100:3110',
    'crux:3200:3210',
  ],
  // seeds: [{
  //   // a url pointing to your server
  //   host: 'crux',
  //   httpPort: 3000,
  //   grpcPort: 3010,
  // }],
  // network: 'testnet',
}

let newWalletClientOpts = {
  ...baseClientOpts,
  wallet: {
    mnemonic: null,
    offlineMode: true,
    // mnemonic: 'a Dash wallet mnemonic with funds goes here',
    // unsafeOptions: {
    //   skipSynchronizationBeforeHeight: 650000, // only sync from early-2022
    // },
  },
}

let alphaWalletClientOpts = {
  ...baseClientOpts,
  wallet: {
    // address: 'yiggi1FHTq1dhkPqAdZRG9QmMSbwUikGTX',
    mnemonic: 'seminar idea float purse stick eager tower essay detail sheriff hip unveil',
    unsafeOptions: {
      // skipSynchronizationBeforeHeight: 650000, // only sync from early-2022
      skipSynchronizationBeforeHeight: 790, // devnet Oct 26 2022
    },
  },
}

const walletOptions = {
  mnemonic: 'protect cave garden achieve hand vacant clarify atom finish outer waste sword',
  seeds: [
    // IP or url(s) pointing to your server(s)
    'localhost',
  ],
  // unsafeOptions: {
  //   skipSynchronizationBeforeHeight: 826000,
  // }
}

const wallet = new Dash.Wallet(newWalletClientOpts);


const client = new Dash.Client({
  network: 'local',
  wallet: walletOptions
});

wallet
  .getAccount()
  .then(async (account) => {
    const acctBalance = (await account.getConfirmedBalance()) / 100000000

    console.log('balance:', acctBalance)

    const identity = await client.platform.identities.get(alphaWalletIdentityID)

    const contractDocuments = {
      note: {
        type: 'object',
        properties: {
          message: {
            type: 'string',
          },
        },
        additionalProperties: false,
      },
    }

    const contract = await client.platform.contracts.create(contractDocuments, identity)

    console.dir({ contract })

    // Make sure contract passes validation checks
    await client.platform.dpp.initialize()
    const validationResult = await client.platform.dpp.dataContract.validate(contract)

    if (validationResult.isValid()) {
      console.log('Validation passed, broadcasting contract..')
      // Sign and submit the data contract
      return await client.platform.contracts.publish(contract, identity)
    }

    account.on(EVENTS.GENERATED_ADDRESS, () => logger.info('GENERATED_ADDRESS'));
    account.on(EVENTS.CONFIRMED_BALANCE_CHANGED, (info) => logger.info('CONFIRMED_BALANCE_CHANGED', info));
    account.on(EVENTS.UNCONFIRMED_BALANCE_CHANGED, (info) => logger.info('UNCONFIRMED_BALANCE_CHANGED', info));
    account.on(EVENTS.BLOCKHEIGHT_CHANGED, (info) => logger.info('BLOCKHEIGHT_CHANGED:', info));
    account.on(EVENTS.PREFETCHED, () => logger.info('PREFETCHED', EVENTS.PREFETCHED));
    account.on(EVENTS.DISCOVERY_STARTED, () => logger.info(EVENTS.PREFETCHED));
  }).catch((e) => {
    console.log('Failed with error', e);
  });

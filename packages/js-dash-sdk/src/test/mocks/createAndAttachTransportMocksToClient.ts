import { Transaction } from '@dashevo/dashcore-lib';
import DAPIClient from '@dashevo/dapi-client';
import { StateTransitionTypes } from '@dashevo/wasm-dpp';

import { createFakeInstantLock } from '../../utils/createFakeIntantLock';
import getResponseMetadataFixture from '../fixtures/getResponseMetadataFixture';
import { createDapiClientMock } from './createDapiClientMock';

import { wait } from '../../utils/wait';

const GetIdentityResponse = require('@dashevo/dapi-client/lib/methods/platform/getIdentity/GetIdentityResponse');
const NotFoundError = require('@dashevo/dapi-client/lib/transport/GrpcTransport/errors/NotFoundError');

const TxStreamMock = require('@dashevo/wallet-lib/src/test/mocks/TxStreamMock');
const TxStreamDataResponseMock = require('@dashevo/wallet-lib/src/test/mocks/TxStreamDataResponseMock');
const TransportMock = require('@dashevo/wallet-lib/src/test/mocks/TransportMock');

function makeTxStreamEmitISLocksForTransactions(transportMock, txStreamMock) {
  transportMock.sendTransaction.callsFake((txString) => {
    const transaction = new Transaction(txString);
    const isLock = createFakeInstantLock(transaction.hash);

    setImmediate(() => {
      // Emit IS lock for the transaction
      txStreamMock.emit(
        TxStreamMock.EVENTS.data,
        new TxStreamDataResponseMock(
          { instantSendLockMessages: [isLock.toBuffer()] },
        ),
      );
    });

    // Emit the same transaction back to the client so it will know about the change transaction
    txStreamMock.emit(
      TxStreamMock.EVENTS.data,
      new TxStreamDataResponseMock(
        { rawTransactions: [transaction.toBuffer()] },
      ),
    );

    return transaction.hash;
  });
}

/**
 * Makes stub remember the identity from the ST and respond with it
 * @param {Client} client
 * @param dapiClientMock
 */
async function makeGetIdentityRespondWithIdentity(client, dapiClientMock) {
  dapiClientMock.platform.broadcastStateTransition.callsFake(async (stBuffer) => {
    const interceptedIdentityStateTransition = await client
      .platform.dpp.stateTransition.createFromBuffer(stBuffer);

    if (interceptedIdentityStateTransition.getType() === StateTransitionTypes.IdentityCreate) {
      const identityToResolve = await client
        .platform.dpp.identity.create(
          interceptedIdentityStateTransition.getIdentityId(),
          interceptedIdentityStateTransition
            .getPublicKeys(),
        );

      identityToResolve.setBalance(
        interceptedIdentityStateTransition.getAssetLockProof().getOutput().satoshis,
      );

      dapiClientMock.platform.getIdentity.withArgs(identityToResolve.getId())
        .resolves(new GetIdentityResponse(
          identityToResolve.toBuffer(),
          getResponseMetadataFixture(),
        ));
    }
  });
}

export async function createAndAttachTransportMocksToClient(client, sinon) {
  await client.platform.initialize();

  const txStreamMock = new TxStreamMock();
  const transportMock = new TransportMock(sinon, txStreamMock);
  const dapiClientMock = createDapiClientMock(sinon);

  // Mock wallet-lib transport to intercept transactions
  client.wallet.transport = transportMock;
  // Mock dapi client for platform endpoints
  client.dapiClient = dapiClientMock;
  client.platform.fetcher.dapiClient = dapiClientMock;
  client.platform.nonceManager.dapiClient = dapiClientMock;

  // Starting account sync
  const accountPromise = client.wallet.getAccount();
  // Breaking the event loop to emit an event
  await wait(0);

  // Simulate headers sync finish
  const { blockHeadersProvider } = client.wallet.transport.client;
  blockHeadersProvider.emit(DAPIClient.BlockHeadersProvider.EVENTS.HISTORICAL_DATA_OBTAINED);
  await wait(0);

  // Emitting TX stream end event to mark finish of the tx sync
  txStreamMock.emit(TxStreamMock.EVENTS.end);

  // Wait for account to resolve
  await accountPromise;

  // Putting data in transport stubs
  transportMock.getIdentityByPublicKeyHash
    .rejects(new NotFoundError('Identity not found', {}, null));
  makeTxStreamEmitISLocksForTransactions(transportMock, txStreamMock);
  await makeGetIdentityRespondWithIdentity(client, dapiClientMock);

  return { txStreamMock, transportMock, dapiClientMock };
}

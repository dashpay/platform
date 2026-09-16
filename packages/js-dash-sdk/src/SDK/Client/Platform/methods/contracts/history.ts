import { DataContract, Identifier } from '@dashevo/wasm-dpp';
import {
  GetDataContractHistoryResponse,
} from '@dashevo/dapi-client/lib/methods/platform/getDataContractHistory/GetDataContractHistoryResponse';
import { Platform } from '../../Platform';
import { PlatformProofVerificationUnavailableError } from '../../../../../errors/PlatformProofVerificationUnavailableError';

const NotFoundError = require('@dashevo/dapi-client/lib/transport/GrpcTransport/errors/NotFoundError');
const InvalidResponseError = require('@dashevo/dapi-client/lib/methods/platform/response/errors/InvalidResponseError');

declare type ContractIdentifier = string | Identifier;

/**
 * Get contracts from the platform
 *
 * @param {ContractIdentifier} identifier - identifier of the contract to fetch
 * @param {bigint} startAtMs
 * @param {number} limit
 * @param {number} offset
 * @returns contracts
 */
export async function history(
  this: Platform,
  identifier: ContractIdentifier,
  startAtMs: bigint,
  limit: number,
  offset: number,
): Promise<any> {
  this.logger.debug(`[Contracts#history] Get Data Contract History for "${identifier}"`);
  await this.initialize();

  const proofVerifier = this.client.getPlatformProofVerifier();
  if (!proofVerifier) {
    throw new PlatformProofVerificationUnavailableError('Data contract history');
  }

  const contractId : Identifier = Identifier.from(identifier);

  const dataContractHistoryResponse = await this.fetcher
    .fetchDataContractHistory(contractId, startAtMs, limit, offset, true);
  this.logger.silly(`[Contracts#history] Fetched Data Contract History for "${identifier}"`);

  if (!dataContractHistoryResponse.getProof() || !dataContractHistoryResponse.getMetadata()) {
    throw new InvalidResponseError('Verified data contract history is missing proof or metadata');
  }

  const dataContractHistory = await proofVerifier.verifyDataContractHistory({
    contractId: contractId.toBuffer(),
    startAtMs,
    limit,
    offset,
    response: dataContractHistoryResponse,
    network: this.client.network,
    protocolVersion: this.protocolVersion!,
  });
  if (!Array.isArray(dataContractHistory)) {
    throw new InvalidResponseError('Proof verifier returned invalid data contract history');
  }

  const contractHistory: { [key: number]: DataContract } = {};

  // eslint-disable-next-line no-restricted-syntax
  for (const dataContractHistoryEntry of dataContractHistory) {
    contractHistory[Number(dataContractHistoryEntry.date)] = await this.dpp
      .dataContract.createFromBuffer(dataContractHistoryEntry.value);
  }

  this.logger.debug(`[Contracts#history] Obtained Data Contract history for "${identifier}"`);

  return contractHistory;
}

/**
 * Fetch data contract history without authenticating it against Platform state.
 *
 * The selected DAPI endpoint controls this result. Prefer `history` with an
 * authenticated `platformProofVerifier` for security-sensitive uses.
 */
export async function historyUnproved(
  this: Platform,
  identifier: ContractIdentifier,
  startAtMs: bigint,
  limit: number,
  offset: number,
): Promise<any> {
  this.logger.debug(
    `[Contracts#historyUnproved] Get unproved Data Contract History for "${identifier}"`,
  );
  await this.initialize();

  const contractId : Identifier = Identifier.from(identifier);
  let dataContractHistoryResponse: GetDataContractHistoryResponse;
  try {
    dataContractHistoryResponse = await this.fetcher
      .fetchDataContractHistory(contractId, startAtMs, limit, offset, false);
  } catch (e) {
    if (e instanceof NotFoundError) {
      return null;
    }

    throw e;
  }

  const contractHistory: { [key: number]: DataContract } = {};
  // eslint-disable-next-line no-restricted-syntax
  for (const entry of dataContractHistoryResponse.getDataContractHistory()) {
    contractHistory[Number(entry.getDate().toString())] = await this.dpp
      .dataContract.createFromBuffer(entry.getValue() as Uint8Array);
  }

  return contractHistory;
}

export default history;

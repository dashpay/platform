import { Identifier } from '@dashevo/wasm-dpp';
import { GetDataContractResponse } from '@dashevo/dapi-client/lib/methods/platform/getDataContract/GetDataContractResponse';
import { Platform } from '../../Platform';

const NotFoundError = require('@dashevo/dapi-client/lib/transport/GrpcTransport/errors/NotFoundError');

declare type ContractIdentifier = string | Identifier;

/**
 * Get contracts from the platform
 *
 * @param {Platform} this - bound instance class
 * @param {ContractIdentifier} identifier - identifier of the contract to fetch
 * @returns contracts
 */
export async function get(this: Platform, identifier: ContractIdentifier): Promise<any> {
  this.logger.debug(`[Contracts#get] Get Data Contract "${identifier}"`);
  await this.initialize();

  const contractId : Identifier = Identifier.from(identifier);

  // Try to get contract from the cache
  // eslint-disable-next-line
  for (const appName of this.client.getApps().getNames()) {
    const appDefinition = this.client.getApps().get(appName);
    if (appDefinition.contractId.equals(contractId) && appDefinition.contract) {
      return appDefinition.contract;
    }
  }

  // If wasm-sdk is available, delegate to it
  if (this.wasmSdk && this.getAdapter()) {
    const adapter = this.getAdapter()!;
    const contractIdString = contractId.toString();
    const cacheKey = `dataContract:${contractIdString}`;
    
    try {
      // Use cached query for better performance
      const result = await adapter.cachedQuery(cacheKey, async () => {
        const sdk = await adapter.getSdk();
        // data_contract_fetch is a standalone function from wasm-sdk
        const wasmAdapter = adapter as any;
        return await wasmAdapter.wasmSdk.data_contract_fetch(sdk, contractIdString);
      });
      
      if (!result) {
        return null;
      }

      // Convert wasm-sdk response to js-dash-sdk format
      const contract = adapter.convertResponse(result, 'dataContract');
      
      // Store contract to the cache
      // eslint-disable-next-line
      for (const appName of this.client.getApps().getNames()) {
        const appDefinition = this.client.getApps().get(appName);
        if (appDefinition.contractId.equals(contractId)) {
          appDefinition.contract = contract;
        }
      }
      
      this.logger.debug(`[Contracts#get] Obtained Data Contract "${identifier}"`);
      
      return contract;
    } catch (e) {
      if (e.message?.includes('not found') || e.message?.includes('does not exist')) {
        return null;
      }
      throw e;
    }
  }

  // Legacy implementation - will be removed once migration is complete
  // Fetch contract otherwise
  let dataContractResponse: GetDataContractResponse;
  try {
    dataContractResponse = await this.fetcher.fetchDataContract(contractId);
    this.logger.silly(`[Contracts#get] Fetched Data Contract "${identifier}"`);
  } catch (e) {
    if (e instanceof NotFoundError) {
      return null;
    }

    throw e;
  }

  const contract = await this.dpp.dataContract
    .createFromBuffer(dataContractResponse.getDataContract() as Uint8Array);

  // Store contract to the cache
  // eslint-disable-next-line
  for (const appName of this.client.getApps().getNames()) {
    const appDefinition = this.client.getApps().get(appName);
    if (appDefinition.contractId.equals(contractId)) {
      appDefinition.contract = contract;
    }
  }

  this.logger.debug(`[Contracts#get] Obtained Data Contract "${identifier}"`);

  return contract;
}

export default get;
import * as wasm from './wasm';
import { asJsonString } from './util';

export class TokensFacade {
  private _sdk: wasm.WasmSdk;

  constructor(sdk: wasm.WasmSdk) {
    this._sdk = sdk;
  }

  // Queries
  priceByContract(contractId: string, tokenPosition: number): Promise<any> {
    return wasm.get_token_price_by_contract(this._sdk, contractId, tokenPosition);
  }

  totalSupply(tokenId: string): Promise<any> {
    return wasm.get_token_total_supply(this._sdk, tokenId);
  }

  totalSupplyWithProof(tokenId: string): Promise<any> {
    return wasm.get_token_total_supply_with_proof_info(this._sdk, tokenId);
  }

  statuses(tokenIds: string[]): Promise<any> {
    return wasm.get_token_statuses(this._sdk, tokenIds);
  }

  statusesWithProof(tokenIds: string[]): Promise<any> {
    return wasm.get_token_statuses_with_proof_info(this._sdk, tokenIds);
  }

  balances(identityIds: string[], tokenId: string): Promise<any> {
    return wasm.get_identities_token_balances(this._sdk, identityIds, tokenId);
  }

  balancesWithProof(identityIds: string[], tokenId: string): Promise<any> {
    return wasm.get_identities_token_balances_with_proof_info(this._sdk, identityIds, tokenId);
  }

  identityTokenInfos(identityId: string, tokenIds?: string[] | null, opts: { limit?: number; offset?: number } = {}): Promise<any> {
    const { limit, offset } = opts;
    return wasm.get_identity_token_infos(this._sdk, identityId, tokenIds ?? null, limit ?? null, offset ?? null);
  }

  identitiesTokenInfos(identityIds: string[], tokenId: string): Promise<any> {
    return wasm.get_identities_token_infos(this._sdk, identityIds, tokenId);
  }

  identityTokenInfosWithProof(identityId: string, tokenIds?: string[] | null, opts: { limit?: number; offset?: number } = {}): Promise<any> {
    const { limit, offset } = opts;
    return wasm.get_identity_token_infos_with_proof_info(this._sdk, identityId, tokenIds ?? null, limit ?? null, offset ?? null);
  }

  identitiesTokenInfosWithProof(identityIds: string[], tokenId: string): Promise<any> {
    return wasm.get_identities_token_infos_with_proof_info(this._sdk, identityIds, tokenId);
  }

  directPurchasePrices(tokenIds: string[]): Promise<any> {
    return wasm.get_token_direct_purchase_prices(this._sdk, tokenIds);
  }

  directPurchasePricesWithProof(tokenIds: string[]): Promise<any> {
    return wasm.get_token_direct_purchase_prices_with_proof_info(this._sdk, tokenIds);
  }

  contractInfo(contractId: string): Promise<any> {
    return wasm.get_token_contract_info(this._sdk, contractId);
  }

  contractInfoWithProof(contractId: string): Promise<any> {
    return wasm.get_token_contract_info_with_proof_info(this._sdk, contractId);
  }

  perpetualDistributionLastClaim(identityId: string, tokenId: string): Promise<any> {
    return wasm.get_token_perpetual_distribution_last_claim(this._sdk, identityId, tokenId);
  }

  perpetualDistributionLastClaimWithProof(identityId: string, tokenId: string): Promise<any> {
    return wasm.get_token_perpetual_distribution_last_claim_with_proof_info(this._sdk, identityId, tokenId);
  }

  // Transitions
  mint(args: { contractId: string; tokenPosition: number; amount: number | string | bigint; identityId: string; privateKeyWif: string; recipientId?: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, amount, identityId, privateKeyWif, recipientId, publicNote } = args;
    return this._sdk.tokenMint(contractId, tokenPosition, String(amount), identityId, privateKeyWif, recipientId ?? null, publicNote ?? null);
  }

  burn(args: { contractId: string; tokenPosition: number; amount: number | string | bigint; identityId: string; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, amount, identityId, privateKeyWif, publicNote } = args;
    return this._sdk.tokenBurn(contractId, tokenPosition, String(amount), identityId, privateKeyWif, publicNote ?? null);
  }

  transfer(args: { contractId: string; tokenPosition: number; amount: number | string | bigint; senderId: string; recipientId: string; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, amount, senderId, recipientId, privateKeyWif, publicNote } = args;
    return this._sdk.tokenTransfer(contractId, tokenPosition, String(amount), senderId, recipientId, privateKeyWif, publicNote ?? null);
  }

  freeze(args: { contractId: string; tokenPosition: number; identityToFreeze: string; freezerId: string; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, identityToFreeze, freezerId, privateKeyWif, publicNote } = args;
    return this._sdk.tokenFreeze(contractId, tokenPosition, identityToFreeze, freezerId, privateKeyWif, publicNote ?? null);
  }

  unfreeze(args: { contractId: string; tokenPosition: number; identityToUnfreeze: string; unfreezerId: string; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, identityToUnfreeze, unfreezerId, privateKeyWif, publicNote } = args;
    return this._sdk.tokenUnfreeze(contractId, tokenPosition, identityToUnfreeze, unfreezerId, privateKeyWif, publicNote ?? null);
  }

  destroyFrozen(args: { contractId: string; tokenPosition: number; identityId: string; destroyerId: string; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, identityId, destroyerId, privateKeyWif, publicNote } = args;
    return this._sdk.tokenDestroyFrozen(contractId, tokenPosition, identityId, destroyerId, privateKeyWif, publicNote ?? null);
  }

  setPriceForDirectPurchase(args: { contractId: string; tokenPosition: number; identityId: string; priceType: string; priceData: unknown; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, identityId, priceType, priceData, privateKeyWif, publicNote } = args;
    return this._sdk.tokenSetPriceForDirectPurchase(contractId, tokenPosition, identityId, priceType, asJsonString(priceData)!, privateKeyWif, publicNote ?? null);
  }

  directPurchase(args: { contractId: string; tokenPosition: number; amount: number | string | bigint; identityId: string; totalAgreedPrice?: number | string | bigint | null; privateKeyWif: string }): Promise<any> {
    const { contractId, tokenPosition, amount, identityId, totalAgreedPrice, privateKeyWif } = args;
    return this._sdk.tokenDirectPurchase(contractId, tokenPosition, String(amount), identityId, totalAgreedPrice != null ? String(totalAgreedPrice) : null, privateKeyWif);
  }

  claim(args: { contractId: string; tokenPosition: number; distributionType: string; identityId: string; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, distributionType, identityId, privateKeyWif, publicNote } = args;
    return this._sdk.tokenClaim(contractId, tokenPosition, distributionType, identityId, privateKeyWif, publicNote ?? null);
  }

  configUpdate(args: { contractId: string; tokenPosition: number; configItemType: string; configValue: unknown; identityId: string; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, configItemType, configValue, identityId, privateKeyWif, publicNote } = args;
    return this._sdk.tokenConfigUpdate(contractId, tokenPosition, configItemType, asJsonString(configValue)!, identityId, privateKeyWif, publicNote ?? null);
  }
}

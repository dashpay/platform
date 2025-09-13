import * as wasm from '../wasm.js';
import { asJsonString } from '../util.js';
import type { EvoSDK } from '../sdk.js';

export class TokensFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  // Queries
  priceByContract(contractId: string, tokenPosition: number): Promise<any> {
    return wasm.get_token_price_by_contract(this.sdk.wasm, contractId, tokenPosition);
  }

  totalSupply(tokenId: string): Promise<any> {
    return wasm.get_token_total_supply(this.sdk.wasm, tokenId);
  }

  totalSupplyWithProof(tokenId: string): Promise<any> {
    return wasm.get_token_total_supply_with_proof_info(this.sdk.wasm, tokenId);
  }

  statuses(tokenIds: string[]): Promise<any> {
    return wasm.get_token_statuses(this.sdk.wasm, tokenIds);
  }

  statusesWithProof(tokenIds: string[]): Promise<any> {
    return wasm.get_token_statuses_with_proof_info(this.sdk.wasm, tokenIds);
  }

  balances(identityIds: string[], tokenId: string): Promise<any> {
    return wasm.get_identities_token_balances(this.sdk.wasm, identityIds, tokenId);
  }

  balancesWithProof(identityIds: string[], tokenId: string): Promise<any> {
    return wasm.get_identities_token_balances_with_proof_info(this.sdk.wasm, identityIds, tokenId);
  }

  identityTokenInfos(identityId: string, tokenIds?: string[] | null, opts: { limit?: number; offset?: number } = {}): Promise<any> {
    const { limit, offset } = opts;
    return wasm.get_identity_token_infos(this.sdk.wasm, identityId, tokenIds ?? null, limit ?? null, offset ?? null);
  }

  identitiesTokenInfos(identityIds: string[], tokenId: string): Promise<any> {
    return wasm.get_identities_token_infos(this.sdk.wasm, identityIds, tokenId);
  }

  identityTokenInfosWithProof(identityId: string, tokenIds?: string[] | null, opts: { limit?: number; offset?: number } = {}): Promise<any> {
    const { limit, offset } = opts;
    return wasm.get_identity_token_infos_with_proof_info(this.sdk.wasm, identityId, tokenIds ?? null, limit ?? null, offset ?? null);
  }

  identitiesTokenInfosWithProof(identityIds: string[], tokenId: string): Promise<any> {
    return wasm.get_identities_token_infos_with_proof_info(this.sdk.wasm, identityIds, tokenId);
  }

  directPurchasePrices(tokenIds: string[]): Promise<any> {
    return wasm.get_token_direct_purchase_prices(this.sdk.wasm, tokenIds);
  }

  directPurchasePricesWithProof(tokenIds: string[]): Promise<any> {
    return wasm.get_token_direct_purchase_prices_with_proof_info(this.sdk.wasm, tokenIds);
  }

  contractInfo(contractId: string): Promise<any> {
    return wasm.get_token_contract_info(this.sdk.wasm, contractId);
  }

  contractInfoWithProof(contractId: string): Promise<any> {
    return wasm.get_token_contract_info_with_proof_info(this.sdk.wasm, contractId);
  }

  perpetualDistributionLastClaim(identityId: string, tokenId: string): Promise<any> {
    return wasm.get_token_perpetual_distribution_last_claim(this.sdk.wasm, identityId, tokenId);
  }

  perpetualDistributionLastClaimWithProof(identityId: string, tokenId: string): Promise<any> {
    return wasm.get_token_perpetual_distribution_last_claim_with_proof_info(this.sdk.wasm, identityId, tokenId);
  }

  // Transitions
  mint(args: { contractId: string; tokenPosition: number; amount: number | string | bigint; identityId: string; privateKeyWif: string; recipientId?: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, amount, identityId, privateKeyWif, recipientId, publicNote } = args;
    return this.sdk.wasm.tokenMint(contractId, tokenPosition, String(amount), identityId, privateKeyWif, recipientId ?? null, publicNote ?? null);
  }

  burn(args: { contractId: string; tokenPosition: number; amount: number | string | bigint; identityId: string; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, amount, identityId, privateKeyWif, publicNote } = args;
    return this.sdk.wasm.tokenBurn(contractId, tokenPosition, String(amount), identityId, privateKeyWif, publicNote ?? null);
  }

  transfer(args: { contractId: string; tokenPosition: number; amount: number | string | bigint; senderId: string; recipientId: string; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, amount, senderId, recipientId, privateKeyWif, publicNote } = args;
    return this.sdk.wasm.tokenTransfer(contractId, tokenPosition, String(amount), senderId, recipientId, privateKeyWif, publicNote ?? null);
  }

  freeze(args: { contractId: string; tokenPosition: number; identityToFreeze: string; freezerId: string; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, identityToFreeze, freezerId, privateKeyWif, publicNote } = args;
    return this.sdk.wasm.tokenFreeze(contractId, tokenPosition, identityToFreeze, freezerId, privateKeyWif, publicNote ?? null);
  }

  unfreeze(args: { contractId: string; tokenPosition: number; identityToUnfreeze: string; unfreezerId: string; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, identityToUnfreeze, unfreezerId, privateKeyWif, publicNote } = args;
    return this.sdk.wasm.tokenUnfreeze(contractId, tokenPosition, identityToUnfreeze, unfreezerId, privateKeyWif, publicNote ?? null);
  }

  destroyFrozen(args: { contractId: string; tokenPosition: number; identityId: string; destroyerId: string; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, identityId, destroyerId, privateKeyWif, publicNote } = args;
    return this.sdk.wasm.tokenDestroyFrozen(contractId, tokenPosition, identityId, destroyerId, privateKeyWif, publicNote ?? null);
  }

  setPriceForDirectPurchase(args: { contractId: string; tokenPosition: number; identityId: string; priceType: string; priceData: unknown; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, identityId, priceType, priceData, privateKeyWif, publicNote } = args;
    return this.sdk.wasm.tokenSetPriceForDirectPurchase(contractId, tokenPosition, identityId, priceType, asJsonString(priceData)!, privateKeyWif, publicNote ?? null);
  }

  directPurchase(args: { contractId: string; tokenPosition: number; amount: number | string | bigint; identityId: string; totalAgreedPrice?: number | string | bigint | null; privateKeyWif: string }): Promise<any> {
    const { contractId, tokenPosition, amount, identityId, totalAgreedPrice, privateKeyWif } = args;
    return this.sdk.wasm.tokenDirectPurchase(contractId, tokenPosition, String(amount), identityId, totalAgreedPrice != null ? String(totalAgreedPrice) : null, privateKeyWif);
  }

  claim(args: { contractId: string; tokenPosition: number; distributionType: string; identityId: string; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, distributionType, identityId, privateKeyWif, publicNote } = args;
    return this.sdk.wasm.tokenClaim(contractId, tokenPosition, distributionType, identityId, privateKeyWif, publicNote ?? null);
  }

  configUpdate(args: { contractId: string; tokenPosition: number; configItemType: string; configValue: unknown; identityId: string; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, configItemType, configValue, identityId, privateKeyWif, publicNote } = args;
    return this.sdk.wasm.tokenConfigUpdate(contractId, tokenPosition, configItemType, asJsonString(configValue)!, identityId, privateKeyWif, publicNote ?? null);
  }
}


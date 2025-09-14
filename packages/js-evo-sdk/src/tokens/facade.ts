import * as wasm from '../wasm.js';
import { asJsonString } from '../util.js';
import type { EvoSDK } from '../sdk.js';

export class TokensFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  // Queries
  async priceByContract(contractId: string, tokenPosition: number): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_token_price_by_contract(w, contractId, tokenPosition);
  }

  async totalSupply(tokenId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_token_total_supply(w, tokenId);
  }

  async totalSupplyWithProof(tokenId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_token_total_supply_with_proof_info(w, tokenId);
  }

  async statuses(tokenIds: string[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_token_statuses(w, tokenIds);
  }

  async statusesWithProof(tokenIds: string[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_token_statuses_with_proof_info(w, tokenIds);
  }

  async balances(identityIds: string[], tokenId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_identities_token_balances(w, identityIds, tokenId);
  }

  async balancesWithProof(identityIds: string[], tokenId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_identities_token_balances_with_proof_info(w, identityIds, tokenId);
  }

  async identityTokenInfos(identityId: string, tokenIds: string[], opts: { limit?: number; offset?: number } = {}): Promise<any> {
    const { limit, offset } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_identity_token_infos(w, identityId, tokenIds);
  }

  async identitiesTokenInfos(identityIds: string[], tokenId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_identities_token_infos(w, identityIds, tokenId);
  }

  async identityTokenInfosWithProof(identityId: string, tokenIds: string[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_identity_token_infos_with_proof_info(w, identityId, tokenIds);
  }

  async identitiesTokenInfosWithProof(identityIds: string[], tokenId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_identities_token_infos_with_proof_info(w, identityIds, tokenId);
  }

  async directPurchasePrices(tokenIds: string[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_token_direct_purchase_prices(w, tokenIds);
  }

  async directPurchasePricesWithProof(tokenIds: string[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_token_direct_purchase_prices_with_proof_info(w, tokenIds);
  }

  async contractInfo(contractId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_token_contract_info(w, contractId);
  }

  async contractInfoWithProof(contractId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_token_contract_info_with_proof_info(w, contractId);
  }

  async perpetualDistributionLastClaim(identityId: string, tokenId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_token_perpetual_distribution_last_claim(w, identityId, tokenId);
  }

  async perpetualDistributionLastClaimWithProof(identityId: string, tokenId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_token_perpetual_distribution_last_claim_with_proof_info(w, identityId, tokenId);
  }

  // Transitions
  async mint(args: { contractId: string; tokenPosition: number; amount: number | string | bigint; identityId: string; privateKeyWif: string; recipientId?: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, amount, identityId, privateKeyWif, recipientId, publicNote } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenMint(contractId, tokenPosition, String(amount), identityId, privateKeyWif, recipientId ?? null, publicNote ?? null);
  }

  async burn(args: { contractId: string; tokenPosition: number; amount: number | string | bigint; identityId: string; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, amount, identityId, privateKeyWif, publicNote } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenBurn(contractId, tokenPosition, String(amount), identityId, privateKeyWif, publicNote ?? null);
  }

  async transfer(args: { contractId: string; tokenPosition: number; amount: number | string | bigint; senderId: string; recipientId: string; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, amount, senderId, recipientId, privateKeyWif, publicNote } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenTransfer(contractId, tokenPosition, String(amount), senderId, recipientId, privateKeyWif, publicNote ?? null);
  }

  async freeze(args: { contractId: string; tokenPosition: number; identityToFreeze: string; freezerId: string; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, identityToFreeze, freezerId, privateKeyWif, publicNote } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenFreeze(contractId, tokenPosition, identityToFreeze, freezerId, privateKeyWif, publicNote ?? null);
  }

  async unfreeze(args: { contractId: string; tokenPosition: number; identityToUnfreeze: string; unfreezerId: string; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, identityToUnfreeze, unfreezerId, privateKeyWif, publicNote } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenUnfreeze(contractId, tokenPosition, identityToUnfreeze, unfreezerId, privateKeyWif, publicNote ?? null);
  }

  async destroyFrozen(args: { contractId: string; tokenPosition: number; identityId: string; destroyerId: string; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, identityId, destroyerId, privateKeyWif, publicNote } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenDestroyFrozen(contractId, tokenPosition, identityId, destroyerId, privateKeyWif, publicNote ?? null);
  }

  async setPriceForDirectPurchase(args: { contractId: string; tokenPosition: number; identityId: string; priceType: string; priceData: unknown; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, identityId, priceType, priceData, privateKeyWif, publicNote } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenSetPriceForDirectPurchase(contractId, tokenPosition, identityId, priceType, asJsonString(priceData)!, privateKeyWif, publicNote ?? null);
  }

  async directPurchase(args: { contractId: string; tokenPosition: number; amount: number | string | bigint; identityId: string; totalAgreedPrice?: number | string | bigint | null; privateKeyWif: string }): Promise<any> {
    const { contractId, tokenPosition, amount, identityId, totalAgreedPrice, privateKeyWif } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenDirectPurchase(contractId, tokenPosition, String(amount), identityId, totalAgreedPrice != null ? String(totalAgreedPrice) : null, privateKeyWif);
  }

  async claim(args: { contractId: string; tokenPosition: number; distributionType: string; identityId: string; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, distributionType, identityId, privateKeyWif, publicNote } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenClaim(contractId, tokenPosition, distributionType, identityId, privateKeyWif, publicNote ?? null);
  }

  async configUpdate(args: { contractId: string; tokenPosition: number; configItemType: string; configValue: unknown; identityId: string; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, configItemType, configValue, identityId, privateKeyWif, publicNote } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenConfigUpdate(contractId, tokenPosition, configItemType, asJsonString(configValue)!, identityId, privateKeyWif, publicNote ?? null);
  }
}

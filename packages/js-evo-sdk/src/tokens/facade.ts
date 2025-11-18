import * as wasm from '../wasm.js';
import { asJsonString } from '../util.js';
import type { EvoSDK } from '../sdk.js';

export class TokensFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  async calculateId(contractId: wasm.IdentifierLike, tokenPosition: number): Promise<string> {
    await wasm.ensureInitialized();
    return wasm.WasmSdk.calculateTokenIdFromContract(contractId, tokenPosition);
  }

  // Queries
  async priceByContract(contractId: wasm.IdentifierLike, tokenPosition: number): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenPriceByContract(contractId, tokenPosition);
  }

  async totalSupply(tokenId: wasm.IdentifierLike): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenTotalSupply(tokenId);
  }

  async totalSupplyWithProof(tokenId: wasm.IdentifierLike): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenTotalSupplyWithProofInfo(tokenId);
  }

  async statuses(tokenIds: wasm.IdentifierLike[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenStatuses(tokenIds);
  }

  async statusesWithProof(tokenIds: wasm.IdentifierLike[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenStatusesWithProofInfo(tokenIds);
  }

  async balances(identityIds: wasm.IdentifierLike[], tokenId: wasm.IdentifierLike): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentitiesTokenBalances(identityIds, tokenId);
  }

  async balancesWithProof(identityIds: wasm.IdentifierLike[], tokenId: wasm.IdentifierLike): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentitiesTokenBalancesWithProofInfo(identityIds, tokenId);
  }

  async identityBalances(identityId: wasm.IdentifierLike, tokenIds: wasm.IdentifierLike[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityTokenBalances(identityId, tokenIds);
  }

  async identityBalancesWithProof(identityId: wasm.IdentifierLike, tokenIds: wasm.IdentifierLike[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityTokenBalancesWithProofInfo(identityId, tokenIds);
  }

  async identityTokenInfos(identityId: wasm.IdentifierLike, tokenIds: wasm.IdentifierLike[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityTokenInfos(identityId, tokenIds);
  }

  async identitiesTokenInfos(identityIds: wasm.IdentifierLike[], tokenId: wasm.IdentifierLike): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentitiesTokenInfos(identityIds, tokenId);
  }

  async identityTokenInfosWithProof(identityId: wasm.IdentifierLike, tokenIds: wasm.IdentifierLike[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityTokenInfosWithProofInfo(identityId, tokenIds);
  }

  async identitiesTokenInfosWithProof(identityIds: wasm.IdentifierLike[], tokenId: wasm.IdentifierLike): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentitiesTokenInfosWithProofInfo(identityIds, tokenId);
  }

  async directPurchasePrices(tokenIds: wasm.IdentifierLike[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenDirectPurchasePrices(tokenIds);
  }

  async directPurchasePricesWithProof(tokenIds: wasm.IdentifierLike[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenDirectPurchasePricesWithProofInfo(tokenIds);
  }

  async contractInfo(contractId: wasm.IdentifierLike): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenContractInfo(contractId);
  }

  async contractInfoWithProof(contractId: wasm.IdentifierLike): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenContractInfoWithProofInfo(contractId);
  }

  async perpetualDistributionLastClaim(identityId: wasm.IdentifierLike, tokenId: wasm.IdentifierLike): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenPerpetualDistributionLastClaim(identityId, tokenId);
  }

  async perpetualDistributionLastClaimWithProof(identityId: wasm.IdentifierLike, tokenId: wasm.IdentifierLike): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenPerpetualDistributionLastClaimWithProofInfo(identityId, tokenId);
  }

  // Transitions
  async mint(args: { contractId: wasm.IdentifierLike; tokenPosition: number; amount: number | string | bigint; identityId: wasm.IdentifierLike; privateKeyWif: string; recipientId?: wasm.IdentifierLike; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, amount, identityId, privateKeyWif, recipientId, publicNote } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenMint(contractId, tokenPosition, String(amount), identityId, privateKeyWif, recipientId, publicNote ?? null);
  }

  async burn(args: { contractId: wasm.IdentifierLike; tokenPosition: number; amount: number | string | bigint; identityId: wasm.IdentifierLike; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, amount, identityId, privateKeyWif, publicNote } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenBurn(contractId, tokenPosition, String(amount), identityId, privateKeyWif, publicNote ?? null);
  }

  async transfer(args: { contractId: wasm.IdentifierLike; tokenPosition: number; amount: number | string | bigint; senderId: wasm.IdentifierLike; recipientId: wasm.IdentifierLike; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, amount, senderId, recipientId, privateKeyWif, publicNote } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenTransfer(contractId, tokenPosition, String(amount), senderId, recipientId, privateKeyWif, publicNote ?? null);
  }

  async freeze(args: { contractId: wasm.IdentifierLike; tokenPosition: number; identityToFreeze: wasm.IdentifierLike; freezerId: wasm.IdentifierLike; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, identityToFreeze, freezerId, privateKeyWif, publicNote } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenFreeze(contractId, tokenPosition, identityToFreeze, freezerId, privateKeyWif, publicNote ?? null);
  }

  async unfreeze(args: { contractId: wasm.IdentifierLike; tokenPosition: number; identityToUnfreeze: wasm.IdentifierLike; unfreezerId: wasm.IdentifierLike; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, identityToUnfreeze, unfreezerId, privateKeyWif, publicNote } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenUnfreeze(contractId, tokenPosition, identityToUnfreeze, unfreezerId, privateKeyWif, publicNote ?? null);
  }

  async destroyFrozen(args: { contractId: wasm.IdentifierLike; tokenPosition: number; identityId: wasm.IdentifierLike; destroyerId: wasm.IdentifierLike; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, identityId, destroyerId, privateKeyWif, publicNote } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenDestroyFrozen(contractId, tokenPosition, identityId, destroyerId, privateKeyWif, publicNote ?? null);
  }

  async setPriceForDirectPurchase(args: { contractId: wasm.IdentifierLike; tokenPosition: number; identityId: wasm.IdentifierLike; priceType: string; priceData: unknown; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, identityId, priceType, priceData, privateKeyWif, publicNote } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenSetPriceForDirectPurchase(contractId, tokenPosition, identityId, priceType, asJsonString(priceData)!, privateKeyWif, publicNote ?? null);
  }

  async directPurchase(args: { contractId: wasm.IdentifierLike; tokenPosition: number; amount: number | string | bigint; identityId: wasm.IdentifierLike; totalAgreedPrice?: number | string | bigint | null; privateKeyWif: string }): Promise<any> {
    const { contractId, tokenPosition, amount, identityId, totalAgreedPrice, privateKeyWif } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenDirectPurchase(contractId, tokenPosition, String(amount), identityId, totalAgreedPrice != null ? String(totalAgreedPrice) : null, privateKeyWif);
  }

  async claim(args: { contractId: wasm.IdentifierLike; tokenPosition: number; distributionType: string; identityId: wasm.IdentifierLike; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, distributionType, identityId, privateKeyWif, publicNote } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenClaim(contractId, tokenPosition, distributionType, identityId, privateKeyWif, publicNote ?? null);
  }

  async configUpdate(args: { contractId: wasm.IdentifierLike; tokenPosition: number; configItemType: string; configValue: unknown; identityId: wasm.IdentifierLike; privateKeyWif: string; publicNote?: string }): Promise<any> {
    const { contractId, tokenPosition, configItemType, configValue, identityId, privateKeyWif, publicNote } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenConfigUpdate(contractId, tokenPosition, configItemType, asJsonString(configValue)!, identityId, privateKeyWif, publicNote ?? null);
  }
}

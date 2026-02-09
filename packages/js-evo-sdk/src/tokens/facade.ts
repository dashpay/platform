import * as wasm from '../wasm.js';
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
  async priceByContract(contractId: wasm.IdentifierLike, tokenPosition: number): Promise<wasm.TokenPriceInfo> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenPriceByContract(contractId, tokenPosition);
  }

  async totalSupply(tokenId: wasm.IdentifierLike): Promise<wasm.TokenTotalSupply | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenTotalSupply(tokenId);
  }

  async totalSupplyWithProof(tokenId: wasm.IdentifierLike): Promise<wasm.ProofMetadataResponseTyped<wasm.TokenTotalSupply | null>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenTotalSupplyWithProofInfo(tokenId);
  }

  async statuses(tokenIds: wasm.IdentifierLike[]): Promise<Map<wasm.Identifier, wasm.TokenStatus>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenStatuses(tokenIds);
  }

  async statusesWithProof(tokenIds: wasm.IdentifierLike[]): Promise<wasm.ProofMetadataResponseTyped<Map<wasm.Identifier, wasm.TokenStatus>>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenStatusesWithProofInfo(tokenIds);
  }

  async balances(identityIds: wasm.IdentifierLike[], tokenId: wasm.IdentifierLike): Promise<Map<wasm.Identifier, bigint>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentitiesTokenBalances(identityIds, tokenId);
  }

  async balancesWithProof(identityIds: wasm.IdentifierLike[], tokenId: wasm.IdentifierLike): Promise<wasm.ProofMetadataResponseTyped<Map<wasm.Identifier, bigint>>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentitiesTokenBalancesWithProofInfo(identityIds, tokenId);
  }

  async identityBalances(identityId: wasm.IdentifierLike, tokenIds: wasm.IdentifierLike[]): Promise<Map<wasm.Identifier, bigint>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityTokenBalances(identityId, tokenIds);
  }

  async identityBalancesWithProof(identityId: wasm.IdentifierLike, tokenIds: wasm.IdentifierLike[]): Promise<wasm.ProofMetadataResponseTyped<Map<wasm.Identifier, bigint>>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityTokenBalancesWithProofInfo(identityId, tokenIds);
  }

  async identityTokenInfos(identityId: wasm.IdentifierLike, tokenIds: wasm.IdentifierLike[]): Promise<Map<wasm.Identifier, wasm.IdentityTokenInfo>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityTokenInfos(identityId, tokenIds);
  }

  async identitiesTokenInfos(identityIds: wasm.IdentifierLike[], tokenId: wasm.IdentifierLike): Promise<Map<wasm.Identifier, wasm.IdentityTokenInfo>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentitiesTokenInfos(identityIds, tokenId);
  }

  async identityTokenInfosWithProof(identityId: wasm.IdentifierLike, tokenIds: wasm.IdentifierLike[]): Promise<wasm.ProofMetadataResponseTyped<Map<wasm.Identifier, wasm.IdentityTokenInfo>>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentityTokenInfosWithProofInfo(identityId, tokenIds);
  }

  async identitiesTokenInfosWithProof(identityIds: wasm.IdentifierLike[], tokenId: wasm.IdentifierLike): Promise<wasm.ProofMetadataResponseTyped<Map<wasm.Identifier, wasm.IdentityTokenInfo>>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getIdentitiesTokenInfosWithProofInfo(identityIds, tokenId);
  }

  async directPurchasePrices(tokenIds: wasm.IdentifierLike[]): Promise<Map<wasm.Identifier, wasm.TokenPriceInfo>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenDirectPurchasePrices(tokenIds);
  }

  async directPurchasePricesWithProof(tokenIds: wasm.IdentifierLike[]): Promise<wasm.ProofMetadataResponseTyped<Map<wasm.Identifier, wasm.TokenPriceInfo>>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenDirectPurchasePricesWithProofInfo(tokenIds);
  }

  async contractInfo(contractId: wasm.IdentifierLike): Promise<wasm.TokenContractInfo | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenContractInfo(contractId);
  }

  async contractInfoWithProof(contractId: wasm.IdentifierLike): Promise<wasm.ProofMetadataResponseTyped<wasm.TokenContractInfo | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenContractInfoWithProofInfo(contractId);
  }

  async perpetualDistributionLastClaim(identityId: wasm.IdentifierLike, tokenId: wasm.IdentifierLike): Promise<wasm.RewardDistributionMoment | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenPerpetualDistributionLastClaim(identityId, tokenId);
  }

  async perpetualDistributionLastClaimWithProof(identityId: wasm.IdentifierLike, tokenId: wasm.IdentifierLike): Promise<wasm.ProofMetadataResponseTyped<wasm.RewardDistributionMoment | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getTokenPerpetualDistributionLastClaimWithProofInfo(identityId, tokenId);
  }

  // Transitions
  async mint(options: wasm.TokenMintOptions): Promise<wasm.TokenMintResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenMint(options);
  }

  async burn(options: wasm.TokenBurnOptions): Promise<wasm.TokenBurnResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenBurn(options);
  }

  async transfer(options: wasm.TokenTransferOptions): Promise<wasm.TokenTransferResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenTransfer(options);
  }

  async freeze(options: wasm.TokenFreezeOptions): Promise<wasm.TokenFreezeResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenFreeze(options);
  }

  async unfreeze(options: wasm.TokenUnfreezeOptions): Promise<wasm.TokenUnfreezeResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenUnfreeze(options);
  }

  async destroyFrozen(options: wasm.TokenDestroyFrozenOptions): Promise<wasm.TokenDestroyFrozenResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenDestroyFrozen(options);
  }

  async emergencyAction(options: wasm.TokenEmergencyActionOptions): Promise<wasm.TokenEmergencyActionResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenEmergencyAction(options);
  }

  async setPrice(options: wasm.TokenSetPriceOptions): Promise<wasm.TokenSetPriceResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenSetPrice(options);
  }

  async directPurchase(options: wasm.TokenDirectPurchaseOptions): Promise<wasm.TokenDirectPurchaseResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenDirectPurchase(options);
  }

  async claim(options: wasm.TokenClaimOptions): Promise<wasm.TokenClaimResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenClaim(options);
  }

  async configUpdate(options: wasm.TokenConfigUpdateOptions): Promise<wasm.TokenConfigUpdateResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.tokenConfigUpdate(options);
  }
}

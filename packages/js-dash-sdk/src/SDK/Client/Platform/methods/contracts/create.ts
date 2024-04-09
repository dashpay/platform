import { Platform } from '../../Platform';

/**
 * Create and prepare contracts for the platform
 *
 * @param {Platform} this - bound instance class
 * @param contractDefinitions - contract definitions
 * @param identity - identity
 * @returns created contracts
 */
export async function create(
  this: Platform,
  contractDefinitions: any,
  identity: any,
): Promise<any> {
  this.logger.debug('[Contracts#create] create data contract');

  await this.initialize();

  const identityNonce = await this.nonceManager.bumpIdentityNonce(identity.getId());
  const dataContract = this.dpp.dataContract.create(
    identity.getId(),
    BigInt(identityNonce),
    contractDefinitions,
  );

  this.logger.debug(`[Contracts#create] created data contract "${dataContract.getId()}"`);

  return dataContract;
}

export default create;

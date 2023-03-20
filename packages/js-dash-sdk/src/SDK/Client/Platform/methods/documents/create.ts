import { Platform } from '../../Platform';

declare interface CreateOpts {
  [name:string]: any;
}

/**
 * Create and prepare documents for the platform
 *
 * @param {Platform} this - bound instance class
 * @param {string} typeLocator - type locator
 * @param identity - identity
 * @param {Object} [data] - options
 */
export async function create(
  this: Platform,
  typeLocator: string,
  identity: any,
  data: CreateOpts = {},
): Promise<any> {
  await this.initialize();

  const { dpp } = this;

  const appNames = this.client.getApps().getNames();

  // We can either provide of type `dashpay.profile`
  // or if only one schema provided, of type `profile`.
  const [appName, fieldType] = (typeLocator.includes('.')) ? typeLocator.split('.') : [appNames[0], typeLocator];

  const { contractId } = this.client.getApps().get(appName);

  const dataContract = await this.contracts.get(contractId);

  if (dataContract === null) {
    throw new Error(`Contract ${appName} not found. Ensure contractId ${contractId} is correct.`);
  }

  return dpp.document.create(
    dataContract,
    identity.getId(),
    fieldType,
    data,
  );
}

export default create;

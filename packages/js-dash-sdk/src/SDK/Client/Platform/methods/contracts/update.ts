import { Platform } from '../../Platform';
import broadcastStateTransition from '../../broadcastStateTransition';
import { signStateTransition } from '../../signStateTransition';

/**
 * Publish contract onto the platform
 *
 * @param {Platform} this - bound instance class
 * @param {DataContract} dataContract - contract
 * @param identity - identity
 * @return {DataContractUpdateTransition}
 */
export default async function update(
  this: Platform,
  dataContract: any,
  identity: any,
): Promise<any> {
  await this.initialize();

  const { wasmDpp } = this;

  // Clone contract
  const updatedDataContract = await wasmDpp.dataContract.createFromObject(
    dataContract.toObject(),
  );

  updatedDataContract.incrementVersion();

  const dataContractUpdateTransition = wasmDpp.dataContract
    .createDataContractUpdateTransition(updatedDataContract);

  await signStateTransition(this, dataContractUpdateTransition, identity, 1);

  console.log('Original');
  console.log(dataContractUpdateTransition.toObject());
  console.log(dataContractUpdateTransition.toBuffer().toString('hex'));

  let validationResult = await wasmDpp.stateTransition.validateBasic(dataContractUpdateTransition);
  console.log('Validation', validationResult.isValid(), validationResult.getErrors());

  console.log('Recreated from objet');
  let recreatedFromObject;
  try {
    recreatedFromObject = await wasmDpp
      .stateTransition.createFromObject(dataContractUpdateTransition.toObject());
  } catch (e) {
    console.log(e);
    // console.log();
    e.getErrors().forEach((e) => {
      console.log(e.getOperation());
      console.log(e.getFieldPath());
    });
    throw e;
  }

  console.log(recreatedFromObject.toObject());
  console.log(recreatedFromObject.toBuffer().toString('hex'));
  validationResult = await wasmDpp.stateTransition.validateBasic(recreatedFromObject);
  console.log('Validation', validationResult.isValid(), validationResult.getErrors());

  console.log('Recreated from buffer');
  let recreatedFromBuffer;
  try {
    recreatedFromBuffer = await wasmDpp.stateTransition.createFromBuffer(dataContractUpdateTransition.toBuffer());
  } catch (e) {
    console.log(e);
    e.getErrors().forEach((e) => {
      console.log(e);
      // console.log(e.getFieldPath());
    });
    throw e;
  }

  console.log(recreatedFromBuffer.toObject());
  console.log(recreatedFromBuffer.toBuffer().toString('hex'));
  validationResult = await wasmDpp.stateTransition.validateBasic(recreatedFromBuffer);
  console.log('Validation', validationResult.isValid(), validationResult.getErrors());

  await broadcastStateTransition(this, dataContractUpdateTransition);

  // Update app with updated data contract if available
  // eslint-disable-next-line
  for (const appName of this.client.getApps().getNames()) {
    const appDefinition = this.client.getApps().get(appName);
    if (appDefinition.contractId.equals(updatedDataContract.getId()) && appDefinition.contract) {
      appDefinition.contract = updatedDataContract;
    }
  }

  return dataContractUpdateTransition;
}

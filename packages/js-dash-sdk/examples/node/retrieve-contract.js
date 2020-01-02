import DashJS from '../../src';

const sdkOpts = {
  network: 'testnet'
};
const sdk = new DashJS.SDK(sdkOpts);

const getContract = async function () {
  let platform = sdk.platform;
  await sdk.isReady();

  platform
      .contracts
      .get('2KfMcMxktKimJxAZUeZwYkFUsEcAZhDKEpQs8GMnpUse')
      .then((contract) => {
        console.dir({contract},{depth:5});
      });

};
getContract();

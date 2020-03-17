const DashJS = require('dash');

const sdkOpts = {
  network: 'testnet'
};
const sdk = new DashJS.SDK(sdkOpts);

const getContract = async function () {
  let platform = sdk.platform;
  await sdk.isReady();

  platform
      .contracts
      .get('77w8Xqn25HwJhjodrHW133aXhjuTsTv9ozQaYpSHACE3')
      .then((contract) => {
        console.dir({contract},{depth:5});
      });

};
getContract();

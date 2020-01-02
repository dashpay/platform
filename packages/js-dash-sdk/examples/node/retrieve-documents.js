import DashJS from '../../src';

const sdkOpts = {
  network: 'testnet'
};
const sdk = new DashJS.SDK(sdkOpts);

const getDocuments = async function () {
  let platform = sdk.platform;
  await sdk.isReady();

  const queryOpts = {
    where: [
       ['normalizedLabel', 'startsWith', 'd'],
       ['normalizedParentDomainName', '==', 'dash'],
   ],
  };

  const documents = await platform.documents.get('dpns.domain', queryOpts);
  console.dir({documents},{depth:5});
};
getDocuments();

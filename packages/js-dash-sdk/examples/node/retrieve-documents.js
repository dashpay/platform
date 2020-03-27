const Dash = require('dash');

const clientOpts = {
  network: 'testnet'
};
const client = new Dash.Client(clientOpts);

const getDocuments = async function () {
  let platform = client.platform;
  await client.isReady();

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

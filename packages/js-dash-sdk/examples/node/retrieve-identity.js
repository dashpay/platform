const Dash = require('dash');

const clientOpts = {
  network: 'testnet'
};
const client = new Dash.Client(clientOpts);

const getIdentity = async function () {
  let platform = client.platform;
  await client.isReady();

  platform
      .identities
      .get('3GegupTgRfdN9JMS8R6QXF3B2VbZtiw63eyudh1oMJAk')
      .then((identity) => {
        console.log({identity});
      });

};
getIdentity();

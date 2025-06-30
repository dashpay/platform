const Dash = require('dash');

async function getBalance() {
    try {
        console.log('Initializing Dash client...');
        
        // Initialize client for mainnet
        const client = new Dash.Client({
            network: 'mainnet',
            dapiAddresses: [
                'https://dapi.dash.org:443',
                'https://dapi-1.dash.org:443',
                'https://dapi-2.dash.org:443'
            ]
        });

        const identityId = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
        
        console.log(`Fetching balance for identity: ${identityId}`);
        
        // Get identity
        const identity = await client.platform.identities.get(identityId);
        
        if (identity) {
            console.log('Identity found!');
            console.log(`Balance: ${identity.balance} credits`);
            console.log(`Balance in DASH: ${identity.balance / 100000000} DASH`);
            console.log(`Revision: ${identity.revision}`);
            console.log(`Public keys count: ${identity.publicKeys.length}`);
        } else {
            console.log('Identity not found');
        }
        
        await client.disconnect();
        
    } catch (error) {
        console.error('Error:', error.message);
        console.error('Full error:', error);
    }
}

getBalance();
const fetch = require('node-fetch');

async function testJsonRpc() {
    // Common Dash JSON-RPC endpoints
    const endpoints = [
        'https://insight.dash.org/api',
        'https://api.dash.org/rpc',
        'https://evonet.dash.org/rpc',
    ];

    const identityId = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';

    console.log('Testing JSON-RPC endpoints for balance...\n');

    for (const endpoint of endpoints) {
        console.log(`Testing ${endpoint}...`);
        
        try {
            // Try different methods
            const methods = [
                { method: 'getidentitybalance', params: [identityId] },
                { method: 'platform.getIdentity', params: { id: identityId } },
                { method: 'getaddressbalance', params: [identityId] }
            ];

            for (const rpcMethod of methods) {
                const response = await fetch(endpoint, {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({
                        jsonrpc: '2.0',
                        id: 1,
                        method: rpcMethod.method,
                        params: rpcMethod.params
                    }),
                    timeout: 5000
                });

                console.log(`  ${rpcMethod.method}: ${response.status} ${response.statusText}`);
                
                if (response.ok) {
                    const data = await response.text();
                    console.log(`  Response: ${data.substring(0, 100)}...`);
                }
            }
        } catch (error) {
            console.log(`  Error: ${error.message}`);
        }
        console.log('');
    }

    // Try REST API endpoints
    console.log('\nTesting REST API endpoints...');
    
    const restEndpoints = [
        `https://insight.dash.org/api/addr/${identityId}/balance`,
        `https://explorer.dash.org/api/address/${identityId}`,
        `https://api.dash.org/v1/identity/${identityId}/balance`
    ];

    for (const url of restEndpoints) {
        try {
            console.log(`Testing ${url}...`);
            const response = await fetch(url, { timeout: 5000 });
            console.log(`  Status: ${response.status} ${response.statusText}`);
            
            if (response.ok) {
                const data = await response.text();
                console.log(`  Response: ${data}`);
            }
        } catch (error) {
            console.log(`  Error: ${error.message}`);
        }
    }
}

testJsonRpc();
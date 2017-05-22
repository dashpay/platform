const Transaction = require('bitcore-lib-dash').Transaction;

//pvr: this code to be moved (to bitcore-lib-dash perhaps?)//////
function getSpendingOutput(utxos) {
    let arr = JSON.parse(utxos);
    let res = Math.max.apply(Math, arr.map(function(o) { return o.amount; }))
    let obj = arr.find(function(o) { return o.amount == res; })
}

getTransaction = function(utxos, authHeadAddresss, changeAddr, accData, privateKey) {

    /*pvr: only 1 input used for now
      output with largest available amount is used
      to implement selectCoins algo (or is this already done on protocol level?)
    */
    let arr = utxos;
    let res = Math.max.apply(Math, arr.map(function(o) { return o.amount; }))
    let obj = arr.find(function(o) { return o.amount == res; })

    let utxo = new Transaction.UnspentOutput({
        "address": obj.address,
        "txid": obj.txid,
        "vout": obj.vout,
        "scriptPubKey": obj.scriptPubKey,
        "satoshis": +(obj.amount) * 100000000
    });

    const MIN_FEE = 200000;
    const MIN_SEND_AMT = 500000;

    return new Transaction()
        .from(utxo)
        .to(authHeadAddresss, MIN_SEND_AMT) //pvr: to send full amount in production (min amount just used to not deplete fundedAddr for tests)
        .change(changeAddr)
        .addData(JSON.stringify(accData))
        .fee(MIN_FEE)
        .sign(privateKey)
        .serialize(true);
}

getAccountData = function(username, authHeadAddresss) {
    return {
        action: '',
        type: '',
        accKey: username,
        pubKey: authHeadAddresss
    }
}
//move code end////////////////////////////////////////////////

exports.create = function(fundedAddr, username, authHeadAddresss, privKey) {

    return SDK.Explorer.API.getUTXO(fundedAddr, username)
        .then(utxos => {
            return SDK.Explorer.API.send(getTransaction(
                utxos,
                authHeadAddresss,
                fundedAddr,
                getAccountData(username, authHeadAddresss),
                privKey));
        })
}





/*Temp
if (_u &&
    has(_u, 'username') &&
    has(_u, 'password') &&
    has(_u, 'email')
) {
    let msg = {
        type: "user",
        action: "create",
        user: _u,
        _reqId: uuid.generate.v4()
    };

    self.emitter.once(msg._reqId, function(data) {
        if (data.hasOwnProperty('error') && data.error == null) {
            return resolve(data.message);
        } else {
            return resolve(data.message);
        }
    });
    self.socket.send(JSON.stringify(msg));
}
else {
    res.error = '100 - Missing Params';
    res.result = 'Missing User';
    return resolve(res);
}

console.log(_u.params)
console.log(_u.returns)
console.log({ "query": "mutation{addRootBase(obj:" + _u.params + ")" + _u.returns + "}" })
ax.post('http://localhost:4000/graphql/graphiql',
    { "query": "mutation{add" + _u.base + "(obj:" + _u.params + ")" + _u.returns + "}" })
    .then(function(response) {
        return resolve(response.data.data)
    })
    .catch(function(error) {
        console.log(error.response.data.errors);
    });


*/
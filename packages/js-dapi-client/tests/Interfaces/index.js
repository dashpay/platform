const SDK = require('../../')

SDK.getUser('alice')
    .then(res => {
        return res.result
    })
    .then(userRes => {
        return client.createUser(userRes.uname, userRes.pubkey, )
    })
    .then(createRes => {
        console.log(createRes)
    })



const has = require('../../util/has.js');
const requesterJSON = require('../../util/requesterJSON.js');
const { uuid } = require('khal');

canPing = (mn) => {

    return new Promise(function(resolve, reject) {
        let uri = mn.fullBase + mn.connectorPath + '/status';
        requesterJSON.get(uri)
            .then(function(resp) {
                if ((resp && resp.hasOwnProperty('info'))) {
                    resolve(mn);
                } else {
                    //pvr: some error handling
                }
            })
            .catch(function(err) {
                resolve(false)
                //not pingabe do nothing (perhaps some logging)
            });
    })
}

exports.isPingable = (mnList) => {

    return new Promise(function(resolve, reject) {
        Promise.all(mnList.map(ul => canPing(ul)))
            .then(validList => {
                resolve(validList.filter(i => i != false))
            })
            .catch(err => {
                console.log(err);
            })
    });
}

isSpvAuthenticated = (mn) => {
    return true;
}

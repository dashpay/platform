const _ = require('underscore')

exports.getConnectorCandidate = function() {

    return new Promise(function(resolve, reject) {

        if (SDK.Discover._state !== "ready") {
            SDK.Discover.init()
                .then(isSuccess => {
                    resolve(SDK.Discover.Masternode.validMNList);
                })
                .catch(e => {
                    console.log(e)
                })
        }
        else {
            resolve(SDK.Discover.Masternode.validMNList);
        }
    }).then(validMNList => {
        if (validMNList && validMNList.length > 0) {
            // _.sample(validMNList).ip;
            return validMNList[0]; //temp for dev purposes
        } else {
            console.log('No MN found :( Sadness & emptyness');
        }
    }).catch(err => {
        console.log(err);
    })
    // throw new Error('Discover need to be init first');
}
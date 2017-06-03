//Choose a random insight uri
const { math } = require('khal');

exports.getInsightCandidate = function() {

    return new Promise(function(resolve, reject) {

        if (SDK.Discover._state !== "ready") {
            SDK.Discover.init()
                .then(isSuccess => {
                    resolve(SDK.Discover.Masternode.validMNList)
                })
        }
        else {
            return resolve(SDK.Discover.Masternode.validMNList)
        }
        // .then(Promise.resolve(SDK.Discover.getInsightCandidate.apply(null, arguments))) //pvr: check arguments?

    }).then(validMNList => {
        if (validMNList && validMNList.length > 0) {
            //Select randomnly one of them
            let selectedMNIdx = math.randomBetweenMinAndMax(0, validMNList.length - 1);
            let el = validMNList[selectedMNIdx];
            return { URI: el.fullBase + el.insightPath, idx: selectedMNIdx };
        } else {
            console.log('No MN found :( Sadness & emptyness');
        }
    })
        .catch(err => {
            console.log(err);
        })
    // throw new Error('Discover need to be init first');
}
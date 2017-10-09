const axios = require('axios'),
    SpvUtils = require('../../util/SpvUtils'),
    _ = require('underscore'),
    fs = require('fs')


getStoredMasternodes = () => {
    return new Promise((resolve, reject) => {
        let path = './masterNodeList.dat' //move to config

        if (fs.existsSync(path)) {
            resolve(fs.readFileSync());
        }
        else {
            resolve(null);
        }

        //todo: filter out old/outdated mastnernodes & some other logic?
    })
}

getSeedUris = () => {
    return SDK._config.DISCOVER.DAPI_SEEDS
        .map(n => {
            return `${n.protocol}://${n.base}:${n.port}`
        })
}

getMnListsFromSeeds = () => {

    return new Promise((resolve, reject) => {
        Promise.all(getSeedUris().map(uri => {
            return axios.get(`${uri}/masternodes/list`)
        }))
            .then(res => {
                resolve(res.map(r => { return r.data }));
            })
            .catch(err => {
                console.log(err);
            })
    })

}


const mnCount = 10; //random number of mns to connect to (move to config)
chooseRandomMns = (mnLists) => {
    return mnLists.map(mnList => {
        return _.sample(mnList, Math.round(mnLists.length / mnCount));
    })
}

//todo review switching between trusted insight servers and dapi nodes
//this implementation does not make sense
getMnObjFromInsightSeed = (seed) => {
    return {
        vin: "na",
        status: "ENABLED",
        rank: 1,
        "ip": seed.base + seed.path,
        "protocol": 0,
        "payee": "na",
        "activeseconds": 14556663,
        "lastseen": 1502078628
    }
}

exports.fetcher = () => {
    return new Promise((resolve, reject) => {
        if (SDK._config.useTrustedServer) {
            resolve(SDK._config.DISCOVER.INSIGHT_SEEDS.map(n => getMnObjFromInsightSeed(n)))
        }
        else {
            getStoredMasternodes()
                .then(mns => {
                    if (mns) {
                        resolve(mns);
                    }
                    else {
                        return getMnListsFromSeeds();
                    }
                })
                .then(mnLists => {
                    return SpvUtils.getMnListOnLongestChain(mnLists);
                })
                .then(bestMnList => {
                    return SpvUtils.getSpvValidMns(bestMnList);
                })
                .then(validMnList => {
                    if (validMnList) {
                        resolve(validMnList);
                    }
                    else {
                        reject('No valid MN found');
                    }
                })
                .catch(err => console.log(err))
        }
    })
}
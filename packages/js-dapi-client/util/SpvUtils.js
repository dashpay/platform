var SpvUtils = {
    getMnListOnLongestChain: (mnLists) => {
        //todo: Determine mn on longest chain 
        return new Promise((resolve, reject) => {
            resolve(mnLists[0]);
        })
    },

    getSpvValidMns: (mnList) => {
        //todo: SPV validate based on vin[0] 1000 Dash collateral
        return new Promise((resolve, reject) => {
            resolve(mnList);
        })
    },
}

module.exports = SpvUtils


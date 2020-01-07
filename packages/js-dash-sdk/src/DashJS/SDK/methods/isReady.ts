const isSelfReady = async function(this: any){
    const self = this;
    return new Promise((res)=>{
        let isReadyInterval = setInterval(() => {
            if (self.state.isReady) {
                clearInterval(isReadyInterval);
                res(true);
            }
        }, 100);
    })
}

async function isReady(this: any) {
    const {state, account} = this;
    if (state.isAccountReady && state.isReady) {
        return true;
    };
    const promises = [];
    if(!state.isAccountReady && account){
        // @ts-ignore
        promises.push(account.isReady());
    }
    if(!state.isReady){
        promises.push(isSelfReady.call(this));
    }

    await Promise.all(promises);
    return true;
}
export default isReady;

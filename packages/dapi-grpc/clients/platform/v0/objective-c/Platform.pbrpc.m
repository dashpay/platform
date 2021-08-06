#import "Platform.pbrpc.h"

#import <ProtoRPC/ProtoRPC.h>
#import <RxLibrary/GRXWriter+Immediate.h>

@implementation Platform

// Designated initializer
- (instancetype)initWithHost:(NSString *)host {
  return (self = [super initWithHost:host packageName:@"org.dash.platform.dapi.v0" serviceName:@"Platform"]);
}

// Override superclass initializer to disallow different package and service names.
- (instancetype)initWithHost:(NSString *)host
                 packageName:(NSString *)packageName
                 serviceName:(NSString *)serviceName {
  return [self initWithHost:host];
}

+ (instancetype)serviceWithHost:(NSString *)host {
  return [[self alloc] initWithHost:host];
}


#pragma mark broadcastStateTransition(BroadcastStateTransitionRequest) returns (BroadcastStateTransitionResponse)

- (void)broadcastStateTransitionWithRequest:(BroadcastStateTransitionRequest *)request handler:(void(^)(BroadcastStateTransitionResponse *_Nullable response, NSError *_Nullable error))handler{
  [[self RPCTobroadcastStateTransitionWithRequest:request handler:handler] start];
}
// Returns a not-yet-started RPC object.
- (GRPCProtoCall *)RPCTobroadcastStateTransitionWithRequest:(BroadcastStateTransitionRequest *)request handler:(void(^)(BroadcastStateTransitionResponse *_Nullable response, NSError *_Nullable error))handler{
  return [self RPCToMethod:@"broadcastStateTransition"
            requestsWriter:[GRXWriter writerWithValue:request]
             responseClass:[BroadcastStateTransitionResponse class]
        responsesWriteable:[GRXWriteable writeableWithSingleHandler:handler]];
}
#pragma mark getIdentity(GetIdentityRequest) returns (GetIdentityResponse)

- (void)getIdentityWithRequest:(GetIdentityRequest *)request handler:(void(^)(GetIdentityResponse *_Nullable response, NSError *_Nullable error))handler{
  [[self RPCTogetIdentityWithRequest:request handler:handler] start];
}
// Returns a not-yet-started RPC object.
- (GRPCProtoCall *)RPCTogetIdentityWithRequest:(GetIdentityRequest *)request handler:(void(^)(GetIdentityResponse *_Nullable response, NSError *_Nullable error))handler{
  return [self RPCToMethod:@"getIdentity"
            requestsWriter:[GRXWriter writerWithValue:request]
             responseClass:[GetIdentityResponse class]
        responsesWriteable:[GRXWriteable writeableWithSingleHandler:handler]];
}
#pragma mark getDataContract(GetDataContractRequest) returns (GetDataContractResponse)

- (void)getDataContractWithRequest:(GetDataContractRequest *)request handler:(void(^)(GetDataContractResponse *_Nullable response, NSError *_Nullable error))handler{
  [[self RPCTogetDataContractWithRequest:request handler:handler] start];
}
// Returns a not-yet-started RPC object.
- (GRPCProtoCall *)RPCTogetDataContractWithRequest:(GetDataContractRequest *)request handler:(void(^)(GetDataContractResponse *_Nullable response, NSError *_Nullable error))handler{
  return [self RPCToMethod:@"getDataContract"
            requestsWriter:[GRXWriter writerWithValue:request]
             responseClass:[GetDataContractResponse class]
        responsesWriteable:[GRXWriteable writeableWithSingleHandler:handler]];
}
#pragma mark getDocuments(GetDocumentsRequest) returns (GetDocumentsResponse)

- (void)getDocumentsWithRequest:(GetDocumentsRequest *)request handler:(void(^)(GetDocumentsResponse *_Nullable response, NSError *_Nullable error))handler{
  [[self RPCTogetDocumentsWithRequest:request handler:handler] start];
}
// Returns a not-yet-started RPC object.
- (GRPCProtoCall *)RPCTogetDocumentsWithRequest:(GetDocumentsRequest *)request handler:(void(^)(GetDocumentsResponse *_Nullable response, NSError *_Nullable error))handler{
  return [self RPCToMethod:@"getDocuments"
            requestsWriter:[GRXWriter writerWithValue:request]
             responseClass:[GetDocumentsResponse class]
        responsesWriteable:[GRXWriteable writeableWithSingleHandler:handler]];
}
#pragma mark getIdentitiesByPublicKeyHashes(GetIdentitiesByPublicKeyHashesRequest) returns (GetIdentitiesByPublicKeyHashesResponse)

- (void)getIdentitiesByPublicKeyHashesWithRequest:(GetIdentitiesByPublicKeyHashesRequest *)request handler:(void(^)(GetIdentitiesByPublicKeyHashesResponse *_Nullable response, NSError *_Nullable error))handler{
  [[self RPCTogetIdentitiesByPublicKeyHashesWithRequest:request handler:handler] start];
}
// Returns a not-yet-started RPC object.
- (GRPCProtoCall *)RPCTogetIdentitiesByPublicKeyHashesWithRequest:(GetIdentitiesByPublicKeyHashesRequest *)request handler:(void(^)(GetIdentitiesByPublicKeyHashesResponse *_Nullable response, NSError *_Nullable error))handler{
  return [self RPCToMethod:@"getIdentitiesByPublicKeyHashes"
            requestsWriter:[GRXWriter writerWithValue:request]
             responseClass:[GetIdentitiesByPublicKeyHashesResponse class]
        responsesWriteable:[GRXWriteable writeableWithSingleHandler:handler]];
}
#pragma mark getIdentityIdsByPublicKeyHashes(GetIdentityIdsByPublicKeyHashesRequest) returns (GetIdentityIdsByPublicKeyHashesResponse)

- (void)getIdentityIdsByPublicKeyHashesWithRequest:(GetIdentityIdsByPublicKeyHashesRequest *)request handler:(void(^)(GetIdentityIdsByPublicKeyHashesResponse *_Nullable response, NSError *_Nullable error))handler{
  [[self RPCTogetIdentityIdsByPublicKeyHashesWithRequest:request handler:handler] start];
}
// Returns a not-yet-started RPC object.
- (GRPCProtoCall *)RPCTogetIdentityIdsByPublicKeyHashesWithRequest:(GetIdentityIdsByPublicKeyHashesRequest *)request handler:(void(^)(GetIdentityIdsByPublicKeyHashesResponse *_Nullable response, NSError *_Nullable error))handler{
  return [self RPCToMethod:@"getIdentityIdsByPublicKeyHashes"
            requestsWriter:[GRXWriter writerWithValue:request]
             responseClass:[GetIdentityIdsByPublicKeyHashesResponse class]
        responsesWriteable:[GRXWriteable writeableWithSingleHandler:handler]];
}
#pragma mark waitForStateTransitionResult(WaitForStateTransitionResultRequest) returns (WaitForStateTransitionResultResponse)

- (void)waitForStateTransitionResultWithRequest:(WaitForStateTransitionResultRequest *)request handler:(void(^)(WaitForStateTransitionResultResponse *_Nullable response, NSError *_Nullable error))handler{
  [[self RPCTowaitForStateTransitionResultWithRequest:request handler:handler] start];
}
// Returns a not-yet-started RPC object.
- (GRPCProtoCall *)RPCTowaitForStateTransitionResultWithRequest:(WaitForStateTransitionResultRequest *)request handler:(void(^)(WaitForStateTransitionResultResponse *_Nullable response, NSError *_Nullable error))handler{
  return [self RPCToMethod:@"waitForStateTransitionResult"
            requestsWriter:[GRXWriter writerWithValue:request]
             responseClass:[WaitForStateTransitionResultResponse class]
        responsesWriteable:[GRXWriteable writeableWithSingleHandler:handler]];
}
#pragma mark getConsensusParams(GetConsensusParamsRequest) returns (GetConsensusParamsResponse)

- (void)getConsensusParamsWithRequest:(GetConsensusParamsRequest *)request handler:(void(^)(GetConsensusParamsResponse *_Nullable response, NSError *_Nullable error))handler{
  [[self RPCTogetConsensusParamsWithRequest:request handler:handler] start];
}
// Returns a not-yet-started RPC object.
- (GRPCProtoCall *)RPCTogetConsensusParamsWithRequest:(GetConsensusParamsRequest *)request handler:(void(^)(GetConsensusParamsResponse *_Nullable response, NSError *_Nullable error))handler{
  return [self RPCToMethod:@"getConsensusParams"
            requestsWriter:[GRXWriter writerWithValue:request]
             responseClass:[GetConsensusParamsResponse class]
        responsesWriteable:[GRXWriteable writeableWithSingleHandler:handler]];
}
@end

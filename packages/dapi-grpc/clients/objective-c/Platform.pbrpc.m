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


#pragma mark applyStateTransition(ApplyStateTransitionRequest) returns (ApplyStateTransitionResponse)

- (void)applyStateTransitionWithRequest:(ApplyStateTransitionRequest *)request handler:(void(^)(ApplyStateTransitionResponse *_Nullable response, NSError *_Nullable error))handler{
  [[self RPCToapplyStateTransitionWithRequest:request handler:handler] start];
}
// Returns a not-yet-started RPC object.
- (GRPCProtoCall *)RPCToapplyStateTransitionWithRequest:(ApplyStateTransitionRequest *)request handler:(void(^)(ApplyStateTransitionResponse *_Nullable response, NSError *_Nullable error))handler{
  return [self RPCToMethod:@"applyStateTransition"
            requestsWriter:[GRXWriter writerWithValue:request]
             responseClass:[ApplyStateTransitionResponse class]
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
@end

#import "Platform.pbobjc.h"

#import <ProtoRPC/ProtoService.h>
#import <ProtoRPC/ProtoRPC.h>
#import <RxLibrary/GRXWriteable.h>
#import <RxLibrary/GRXWriter.h>



NS_ASSUME_NONNULL_BEGIN

@protocol Platform <NSObject>

#pragma mark applyStateTransition(ApplyStateTransitionRequest) returns (ApplyStateTransitionResponse)

- (void)applyStateTransitionWithRequest:(ApplyStateTransitionRequest *)request handler:(void(^)(ApplyStateTransitionResponse *_Nullable response, NSError *_Nullable error))handler;

- (GRPCProtoCall *)RPCToapplyStateTransitionWithRequest:(ApplyStateTransitionRequest *)request handler:(void(^)(ApplyStateTransitionResponse *_Nullable response, NSError *_Nullable error))handler;


#pragma mark getIdentity(GetIdentityRequest) returns (GetIdentityResponse)

- (void)getIdentityWithRequest:(GetIdentityRequest *)request handler:(void(^)(GetIdentityResponse *_Nullable response, NSError *_Nullable error))handler;

- (GRPCProtoCall *)RPCTogetIdentityWithRequest:(GetIdentityRequest *)request handler:(void(^)(GetIdentityResponse *_Nullable response, NSError *_Nullable error))handler;


#pragma mark getDataContract(GetDataContractRequest) returns (GetDataContractResponse)

- (void)getDataContractWithRequest:(GetDataContractRequest *)request handler:(void(^)(GetDataContractResponse *_Nullable response, NSError *_Nullable error))handler;

- (GRPCProtoCall *)RPCTogetDataContractWithRequest:(GetDataContractRequest *)request handler:(void(^)(GetDataContractResponse *_Nullable response, NSError *_Nullable error))handler;


#pragma mark getDocuments(GetDocumentsRequest) returns (GetDocumentsResponse)

- (void)getDocumentsWithRequest:(GetDocumentsRequest *)request handler:(void(^)(GetDocumentsResponse *_Nullable response, NSError *_Nullable error))handler;

- (GRPCProtoCall *)RPCTogetDocumentsWithRequest:(GetDocumentsRequest *)request handler:(void(^)(GetDocumentsResponse *_Nullable response, NSError *_Nullable error))handler;


@end

/**
 * Basic service implementation, over gRPC, that only does
 * marshalling and parsing.
 */
@interface Platform : GRPCProtoService<Platform>
- (instancetype)initWithHost:(NSString *)host NS_DESIGNATED_INITIALIZER;
+ (instancetype)serviceWithHost:(NSString *)host;
@end

NS_ASSUME_NONNULL_END

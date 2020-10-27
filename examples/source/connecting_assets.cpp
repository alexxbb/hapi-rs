#include <HAPI/HAPI.h>
#include <iostream>
#include <string>

#define ENSURE_SUCCESS( result ) \
if ( (result) != HAPI_RESULT_SUCCESS ) \
{ \
    std::cout << "Failure at " << __FILE__ << ": " << __LINE__ << std::endl; \
    std::cout << getLastError() << std::endl; \
    exit( 1 ); \
}

#define ENSURE_COOK_SUCCESS( result ) \
if ( (result) != HAPI_RESULT_SUCCESS ) \
{ \
    std::cout << "Failure at " << __FILE__ << ": " << __LINE__ << std::endl; \
    std::cout << getLastCookError() << std::endl; \
    exit( 1 ); \
}

static std::string getLastError();
static std::string getLastCookError();

int
main( int argc, char **argv )
{
    HAPI_CookOptions cookOptions = HAPI_CookOptions_Create();

    HAPI_Session session;

    HAPI_CreateInProcessSession( &session );
    
    ENSURE_SUCCESS( HAPI_Initialize( &session,
				     &cookOptions,
				     true,
				     -1,
				     nullptr,
				     nullptr,
				     nullptr,
				     nullptr,
				     nullptr ) );

    HAPI_NodeId newNode;
    
    ENSURE_SUCCESS( HAPI_CreateInputNode( &session, &newNode, "Cube" ) );
    ENSURE_SUCCESS( HAPI_CookNode ( &session, newNode, &cookOptions ) );

    int cookStatus;
    HAPI_Result cookResult;

    do
    {
	cookResult = HAPI_GetStatus( &session, HAPI_STATUS_COOK_STATE, &cookStatus );
    }
    while (cookStatus > HAPI_STATE_MAX_READY_STATE && cookResult == HAPI_RESULT_SUCCESS);

    ENSURE_SUCCESS( cookResult );
    ENSURE_COOK_SUCCESS( cookStatus );

    // Creating the triangle vertices
    HAPI_PartInfo newNodePart = HAPI_PartInfo_Create();

    newNodePart.type = HAPI_PARTTYPE_MESH;
    newNodePart.faceCount = 6;
    newNodePart.vertexCount = 24;
    newNodePart.pointCount = 8;
    
    ENSURE_SUCCESS( HAPI_SetPartInfo( &session, newNode, 0, &newNodePart ) );

    HAPI_AttributeInfo newNodePointInfo = HAPI_AttributeInfo_Create();
    newNodePointInfo.count = 8;
    newNodePointInfo.tupleSize = 3;
    newNodePointInfo.exists = true;
    newNodePointInfo.storage = HAPI_STORAGETYPE_FLOAT;
    newNodePointInfo.owner = HAPI_ATTROWNER_POINT;

    ENSURE_SUCCESS( HAPI_AddAttribute( &session, newNode, 0, "P", &newNodePointInfo ) );

    float positions[ 24 ] = { 0.0f, 0.0f, 0.0f,   // 0
			      0.0f, 0.0f, 1.0f,   // 1
			      0.0f, 1.0f, 0.0f,   // 2
			      0.0f, 1.0f, 1.0f,   // 3
			      1.0f, 0.0f, 0.0f,   // 4 
			      1.0f, 0.0f, 1.0f,   // 5
			      1.0f, 1.0f, 0.0f,   // 6
			      1.0f, 1.0f, 1.0f }; // 7
    
    ENSURE_SUCCESS( HAPI_SetAttributeFloatData( &session, newNode, 0, "P", &newNodePointInfo, positions, 0, 8 ) );
    
    int vertices[ 24 ] = { 0, 2, 6, 4,
			   2, 3, 7, 6,
			   2, 0, 1, 3,
			   1, 5, 7, 3,
			   5, 4, 6, 7,
			   0, 4, 5, 1 };
    
    ENSURE_SUCCESS( HAPI_SetVertexList( &session, newNode, 0, vertices, 0, 24 ) );
   
    int face_counts [ 6 ] = { 4, 4, 4, 4, 4, 4 };

    ENSURE_SUCCESS( HAPI_SetFaceCounts( &session, newNode, 0, face_counts, 0, 6 ) );
    
    ENSURE_SUCCESS( HAPI_CommitGeo( &session, newNode ) );

    HAPI_NodeId subdivideNode;

    ENSURE_SUCCESS( HAPI_CreateNode( &session, -1, "Sop/subdivide", "Cube Subdivider", true, &subdivideNode ) );
    
    ENSURE_SUCCESS( HAPI_ConnectNodeInput( &session, subdivideNode, 0, newNode ) );

    ENSURE_SUCCESS( HAPI_SaveHIPFile( &session, "otls/connecting_assets.hip", false ) );
    
    HAPI_Cleanup( &session );

    return 0;
}

static std::string
getLastError()
{
    int bufferLength;
    HAPI_GetStatusStringBufLength( nullptr,
				   HAPI_STATUS_CALL_RESULT,
				   HAPI_STATUSVERBOSITY_ERRORS,
				   &bufferLength );

    char * buffer = new char[ bufferLength ];

    HAPI_GetStatusString( nullptr, HAPI_STATUS_CALL_RESULT, buffer, bufferLength );

    std::string result( buffer );
    delete [] buffer;

    return result;
}

static std::string
getLastCookError()
{
    int bufferLength;
    HAPI_GetStatusStringBufLength( nullptr,
				   HAPI_STATUS_COOK_RESULT,
				   HAPI_STATUSVERBOSITY_ERRORS,
				   &bufferLength );

    char * buffer = new char[ bufferLength ];

    HAPI_GetStatusString( nullptr, HAPI_STATUS_COOK_RESULT, buffer, bufferLength );

    std::string result( buffer );
    delete[] buffer;

    return result;
}

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
main( int argc, char **argv)
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

    ENSURE_SUCCESS( HAPI_CreateInputNode( &session, &newNode, "Curve" ) );
    ENSURE_SUCCESS( HAPI_CookNode ( &session, newNode, &cookOptions ) );
    
    int cookStatus;
    HAPI_Result cookResult;

    do
    {
	cookResult = HAPI_GetStatus( &session, HAPI_STATUS_COOK_STATE, &cookStatus );
	std::cout << "Waiting on cook." << std::endl;
    }
    while (cookStatus > HAPI_STATE_MAX_READY_STATE && cookResult == HAPI_RESULT_SUCCESS);

    ENSURE_SUCCESS( cookResult );
    ENSURE_COOK_SUCCESS( cookStatus );

    HAPI_PartInfo newNodePart = HAPI_PartInfo_Create();

    newNodePart.type = HAPI_PARTTYPE_CURVE;
    newNodePart.faceCount = 1;
    newNodePart.vertexCount = 4;
    newNodePart.pointCount = 4;

    ENSURE_SUCCESS( HAPI_SetPartInfo( &session, newNode, 0, &newNodePart ) );

    HAPI_CurveInfo curveInfo;

    curveInfo.curveType = HAPI_CURVETYPE_NURBS;
    curveInfo.curveCount = 1;
    curveInfo.vertexCount = 4;
    curveInfo.knotCount = 8;
    curveInfo.isPeriodic = false;
    curveInfo.order = 4;
    curveInfo.hasKnots = true;

    ENSURE_SUCCESS( HAPI_SetCurveInfo( &session, newNode, 0, &curveInfo ) );

    int curveCount = 4;
    ENSURE_SUCCESS( HAPI_SetCurveCounts( &session, newNode, newNodePart.id, &curveCount, 0, 1 ) );

    float curveKnots[ 8 ] = { 0.0f, 0.0f, 0.0f, 0.0f, 1.0f, 1.0f, 1.0f, 1.0f };
    ENSURE_SUCCESS( HAPI_SetCurveKnots( &session, newNode, newNodePart.id, curveKnots, 0, 8 ) );

    HAPI_AttributeInfo attrInfo = HAPI_AttributeInfo_Create();

    attrInfo.count = 4;
    attrInfo.tupleSize = 3;
    attrInfo.exists = true;
    attrInfo.storage = HAPI_STORAGETYPE_FLOAT;
    attrInfo.owner = HAPI_ATTROWNER_POINT;

    ENSURE_SUCCESS( HAPI_AddAttribute( &session, newNode, 0, "P", &attrInfo ) );

    float positions [ 12 ] = { -4.0f, 0.0f, 4.0f,
			       -4.0f, 0.0f, -4.0f,
			       4.0f, 0.0f, -4.0f,
			       4.0f, 0.0f, 4.0f };

    ENSURE_SUCCESS( HAPI_SetAttributeFloatData( &session, newNode, 0, "P", &attrInfo, positions, 0, 4 ) );

    ENSURE_SUCCESS( HAPI_CommitGeo( &session, newNode ) );
    ENSURE_SUCCESS( HAPI_SaveHIPFile( &session, "otls/curve_marshall.hip", true ) );
    
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


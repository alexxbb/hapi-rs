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
static std::string getString( HAPI_StringHandle stringHandle );

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

    HAPI_NodeId cubeNode;

    ENSURE_SUCCESS( HAPI_CreateInputNode( &session, &cubeNode, "Cube" ) );
    ENSURE_SUCCESS( HAPI_CookNode( &session, cubeNode, &cookOptions ) );

    int cookStatus;
    HAPI_Result cookResult;

    do
    {
	cookResult = HAPI_GetStatus( &session, HAPI_STATUS_COOK_STATE, &cookStatus );
    }
    while (cookStatus > HAPI_STATE_MAX_READY_STATE && cookResult == HAPI_RESULT_SUCCESS );

    ENSURE_SUCCESS( cookResult );
    ENSURE_COOK_SUCCESS( cookStatus );

    HAPI_PartInfo cubePart = HAPI_PartInfo_Create();

    cubePart.type = HAPI_PARTTYPE_MESH;
    cubePart.faceCount = 6;
    cubePart.vertexCount = 24;
    cubePart.pointCount = 8;

    ENSURE_SUCCESS( HAPI_SetPartInfo( &session, cubeNode, 0, &cubePart ) );

    HAPI_AttributeInfo attrInfo = HAPI_AttributeInfo_Create();

    attrInfo.count = 8;
    attrInfo.tupleSize = 3;
    attrInfo.exists = true;
    attrInfo.storage = HAPI_STORAGETYPE_FLOAT;
    attrInfo.owner = HAPI_ATTROWNER_POINT;

    ENSURE_SUCCESS( HAPI_AddAttribute( &session, cubeNode, 0, "P", &attrInfo ) );

    float positions[ 24 ] = { 0.0f, 0.0f, 0.0f,   // 0
			      0.0f, 0.0f, 1.0f,   // 1
			      0.0f, 1.0f, 0.0f,   // 2
			      0.0f, 1.0f, 1.0f,   // 3
			      1.0f, 0.0f, 0.0f,   // 4 
			      1.0f, 0.0f, 1.0f,   // 5
			      1.0f, 1.0f, 0.0f,   // 6
			      1.0f, 1.0f, 1.0f }; // 7
    
    ENSURE_SUCCESS( HAPI_SetAttributeFloatData( &session, cubeNode, 0, "P", &attrInfo, positions, 0, 8 ) );
    
    int vertices[ 24 ] = { 0, 2, 6, 4,
			   2, 3, 7, 6,
			   2, 0, 1, 3,
			   1, 5, 7, 3,
			   5, 4, 6, 7,
			   0, 4, 5, 1 };
    
    ENSURE_SUCCESS( HAPI_SetVertexList( &session, cubeNode, 0, vertices, 0, 24 ) );
   
    int face_counts [ 6 ] = { 4, 4, 4, 4, 4, 4 };

    ENSURE_SUCCESS( HAPI_SetFaceCounts( &session, cubeNode, 0, face_counts, 0, 6 ) );
   
    ENSURE_SUCCESS( HAPI_AddGroup( &session, cubeNode, cubePart.id, HAPI_GROUPTYPE_POINT, "pointGroup" ) );
    
    int groupElementCount = HAPI_PartInfo_GetElementCountByGroupType( &cubePart, HAPI_GROUPTYPE_POINT );

    int * pointMembership = new int[ groupElementCount ];
    for ( int ii = 0; ii < groupElementCount; ++ii )
    {
	if ( ii % 2 )
	{
	    pointMembership[ ii ] = 1;
	}
	else
	{
	    pointMembership[ ii ] = 0; 
	}
    }
    
    ENSURE_SUCCESS( HAPI_SetGroupMembership( &session, cubeNode, cubePart.id, HAPI_GROUPTYPE_POINT,
					     "pointGroup", pointMembership, 0, groupElementCount ) );
    
    ENSURE_SUCCESS( HAPI_CommitGeo( &session, cubeNode ) );
    
    HAPI_NodeId xformNode;
    ENSURE_SUCCESS( HAPI_CreateNode( &session, -1, "Sop/xform", "PointGroupManipulator", false, &xformNode ) );

    ENSURE_SUCCESS( HAPI_ConnectNodeInput( &session, xformNode, 0, cubeNode ) );

    // Set xform parameters

    HAPI_NodeInfo xformInfo;
    ENSURE_SUCCESS( HAPI_GetNodeInfo( &session, xformNode, &xformInfo ) );

    HAPI_ParmInfo * parmInfos = new HAPI_ParmInfo[ xformInfo.parmCount ];
    ENSURE_SUCCESS( HAPI_GetParameters( &session, xformNode, parmInfos, 0, xformInfo.parmCount ) );

    int groupParmIndex = -1;
    int tParmIndex = -1;
    
    for ( int ii = 0; ii < xformInfo.parmCount; ++ii )
    {
	std::string parmName = getString( parmInfos[ ii ].nameSH );
	if ( parmName == "group" )
	{
	    groupParmIndex = ii;
	}
	if ( parmName == "t" )
	{
	    tParmIndex = ii;
	}
    }

    if ( groupParmIndex < 0 || tParmIndex < 0 )
    {
	std::cout << "Error: couldn't find required parameters group or t" << std::endl;
	exit( 1 );
    }

    float tParmValue[ 3 ] = { 0.0f, 1.0f, 0.0f };

    ENSURE_SUCCESS( HAPI_SetParmFloatValues( &session, xformNode, tParmValue, parmInfos[ tParmIndex ].floatValuesIndex, 3 ) );

    ENSURE_SUCCESS( HAPI_SetParmStringValue( &session, xformNode, "pointGroup", groupParmIndex, 0 ) );

    ENSURE_SUCCESS( HAPI_CookNode( &session, xformNode, &cookOptions ) );

    int xformCookStatus;
    HAPI_Result xformCookResult;

    do
    {
	xformCookResult = HAPI_GetStatus( &session, HAPI_STATUS_COOK_STATE, &xformCookStatus );
    }
    while (xformCookStatus > HAPI_STATE_MAX_READY_STATE && xformCookResult == HAPI_RESULT_SUCCESS );

    ENSURE_SUCCESS( xformCookResult );
    ENSURE_COOK_SUCCESS( xformCookStatus );
    
    ENSURE_SUCCESS( HAPI_SaveHIPFile( &session, "otls/groups.hip", false ) );

    HAPI_GeoInfo xformGeoInfo;
    ENSURE_SUCCESS( HAPI_GetGeoInfo( &session, xformNode, &xformGeoInfo ) );

    int numGroups = HAPI_GeoInfo_GetGroupCountByType( &xformGeoInfo, HAPI_GROUPTYPE_POINT );

    std::cout << "Number of point groups on xform: " << numGroups << std::endl;

    HAPI_PartInfo partInfo;
    ENSURE_SUCCESS( HAPI_GetPartInfo( &session, xformNode, 0, &partInfo ) );

    int numElementsInGroup = HAPI_PartInfo_GetElementCountByGroupType( &partInfo, HAPI_GROUPTYPE_POINT );

    std::cout << numElementsInGroup << " points in pointGroup" << std::endl;

    int * membership = new int[ numElementsInGroup ];

    ENSURE_SUCCESS( HAPI_GetGroupMembership( &session, xformNode, partInfo.id, HAPI_GROUPTYPE_POINT,
					     "pointGroup", nullptr, membership, 0, numElementsInGroup ) );

    for ( int ii = 0; ii < numElementsInGroup; ++ii )
    {
	if ( membership[ ii ] )
	{
	    std::cout << "Point " << ii << " is in pointGroup" << std::endl;
	}
    }
    
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

static std::string
getString( HAPI_StringHandle stringHandle )
{
    if ( stringHandle == 0 )
    {
	return "";
    }

    int bufferLength;
    HAPI_GetStringBufLength( nullptr,
				   stringHandle,
				   &bufferLength );

    char * buffer = new char[ bufferLength ];

    HAPI_GetString ( nullptr, stringHandle, buffer, bufferLength );

    std::string result( buffer );
    delete [] buffer;

    return result;
}

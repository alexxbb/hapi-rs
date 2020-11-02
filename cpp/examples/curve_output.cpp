#include <HAPI/HAPI.h>
#include <iostream>
#include <string>
#include <vector>

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
static void printCurveInfo ( HAPI_ObjectInfo &objInfo, HAPI_GeoInfo &geoInfo, HAPI_PartInfo &partInfo);

int
main( int argc, char **argv)
{
    const char * hdaFile = argc == 2 ? argv[ 1 ] : "otls/nurbs_curve.hda";
    
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

    HAPI_AssetLibraryId assetLibId;
    ENSURE_SUCCESS( HAPI_LoadAssetLibraryFromFile( &session, hdaFile, true, &assetLibId ) );
   
    int assetCount;
    ENSURE_SUCCESS( HAPI_GetAvailableAssetCount( &session, assetLibId, &assetCount ) );

    if (assetCount > 1)
    {
	std::cout << "Should only be loading 1 asset here" << std::endl;
	exit ( 1 );
    }
    
    HAPI_StringHandle assetSh;
    ENSURE_SUCCESS( HAPI_GetAvailableAssets( &session, assetLibId, &assetSh, assetCount ) );

    std::string assetName = getString( assetSh );

    HAPI_NodeId nodeId;
    ENSURE_SUCCESS( HAPI_CreateNode( &session, -1, assetName.c_str(), "Loaded Asset", false, &nodeId ) );
    
    ENSURE_SUCCESS( HAPI_CookNode( &session, nodeId, &cookOptions ) );
    
    int cookStatus;
    HAPI_Result cookResult;

    do
    {
	cookResult = HAPI_GetStatus( &session, HAPI_STATUS_COOK_STATE, &cookStatus );
    }
    while (cookStatus > HAPI_STATE_MAX_READY_STATE && cookResult == HAPI_RESULT_SUCCESS);

    ENSURE_SUCCESS( cookResult );
    ENSURE_COOK_SUCCESS( cookStatus );

    HAPI_NodeInfo nodeInfo;
    ENSURE_SUCCESS( HAPI_GetNodeInfo( &session, nodeId, &nodeInfo ) );

    HAPI_ObjectInfo objInfo;
    ENSURE_SUCCESS( HAPI_GetObjectInfo( &session, nodeId, &objInfo ) );
    
    int childCount;
    ENSURE_SUCCESS( HAPI_ComposeChildNodeList( &session, nodeId, HAPI_NODETYPE_SOP, HAPI_NODEFLAGS_SOP_CURVE, true, &childCount ) );

    HAPI_NodeId *nodeChildren = new HAPI_NodeId[ childCount ];
    ENSURE_SUCCESS( HAPI_GetComposedChildNodeList( &session, nodeId, nodeChildren, childCount ) );
    
    for ( int i = 0; i < childCount; ++i )
    {
	HAPI_NodeId &child = nodeChildren[ i ];
	
	HAPI_NodeInfo info;
	ENSURE_SUCCESS( HAPI_GetNodeInfo( &session, child, &info ) );

	if ( info.type != HAPI_NODETYPE_SOP )
	{
	    continue;
	}

	HAPI_GeoInfo geoInfo;
	ENSURE_SUCCESS( HAPI_GetGeoInfo( &session, child, &geoInfo ) );
	    
	for ( int partIndex = 0; partIndex < geoInfo.partCount; ++partIndex )
	{
	    HAPI_PartInfo partInfo;
	    ENSURE_SUCCESS( HAPI_GetPartInfo( &session, child, partIndex, &partInfo ) );
	    
	    if ( partInfo.type == HAPI_PARTTYPE_CURVE )
	    {
		printCurveInfo( objInfo, geoInfo, partInfo );
	    }
	}
	
    }

    char in;
    std::cout << "Enter some input to exit" << std::endl;
    std::cin >> in;
    
    HAPI_Cleanup( &session );

    return 0;
}

static void printCurveInfo ( HAPI_ObjectInfo &objInfo, HAPI_GeoInfo &geoInfo, HAPI_PartInfo &partInfo)
{
    std::cout << "Object Node: " << objInfo.nodeId << ", Geometry Node: " << geoInfo.nodeId <<
	", Part ID: " << partInfo.id << std::endl;

    HAPI_CurveInfo curveInfo;
    ENSURE_SUCCESS( HAPI_GetCurveInfo( nullptr, geoInfo.nodeId, partInfo.id, &curveInfo ) );

    if ( curveInfo.curveType == HAPI_CURVETYPE_LINEAR )
	std::cout << "curve mesh type = Linear" << std::endl;
    else if ( curveInfo.curveType == HAPI_CURVETYPE_BEZIER )
	std::cout << "curve mesh type = Bezier" << std::endl;
    else if ( curveInfo.curveType == HAPI_CURVETYPE_NURBS )
	std::cout << "curve mesh type = Nurbs" << std::endl;
    else 
	std::cout << "curve mesh type = Unknown" << std::endl;

    std::cout << "curve count: " << curveInfo.curveCount << std::endl;

    int vertexOffset = 0, knotOffset = 0;
    
    for ( int curveIndex = 0; curveIndex < curveInfo.curveCount; ++curveIndex )
    {
	std::cout << "Curve " << curveIndex + 1 << " of " << curveInfo.curveCount << std::endl;

	// Number of control vertices
	
	int numVertices;
	ENSURE_SUCCESS( HAPI_GetCurveCounts( nullptr, geoInfo.nodeId, partInfo.id, &numVertices, curveIndex, 1 ) );

	std::cout << "Number of vertices : " << numVertices << std::endl;

	// Order of this particular curve

	int order;

	if ( curveInfo.order != HAPI_CURVE_ORDER_VARYING
	     && curveInfo.order != HAPI_CURVE_ORDER_INVALID )
	{
	    order = curveInfo.order;
	}
	else
	{
	    ENSURE_SUCCESS( HAPI_GetCurveOrders( nullptr, geoInfo.nodeId, partInfo.id, &order, curveIndex, 1 ) );
	}

	std::cout << "Curve Order: " << order << std::endl;

	// If there's not enough vertices, then don't try to create the curve.
	if ( numVertices < order )
	{
	    std::cout << "Not enough vertcies on curve " << curveIndex + 1 << " of "
		      << curveInfo.curveCount << ": skipping to next curve" << std::endl;

	    // The curve at curveIndex will have numVertices vertices, and may have
	    // some knots. The knot count will be numVertices + order for
	    // nurbs curves.
	    vertexOffset += numVertices * 4;
	    knotOffset += numVertices + order;

	    continue;
	}

	HAPI_AttributeInfo attrInfoP;
	ENSURE_SUCCESS( HAPI_GetAttributeInfo( nullptr, geoInfo.nodeId, partInfo.id, "P", HAPI_ATTROWNER_POINT, &attrInfoP ) );

	std::vector< float > pArray( attrInfoP.count * attrInfoP.tupleSize );
	ENSURE_SUCCESS( HAPI_GetAttributeFloatData( nullptr, geoInfo.nodeId, partInfo.id, "P", &attrInfoP,
						    -1, &pArray.front(), 0, attrInfoP.count ) );

	for ( int j = 0; j < numVertices; j++ )
        {
	    std::cout << "CV " << j + 1 << ": " << pArray[ j * 3 + 0 ] << ","
		      << pArray[ j * 3 + 1 ] << ","
		      << pArray[ j * 3 + 2 ] << std::endl;
	}
	if ( curveInfo.hasKnots)
        {
	    std::vector< float > knots;
	    knots.resize( numVertices + order );

	    ENSURE_SUCCESS( HAPI_GetCurveKnots( nullptr, geoInfo.nodeId, partInfo.id, &knots.front(),
						knotOffset, numVertices + order ) );
	    
	    for( int j = 0; j < numVertices + order; j++ )
		{
		    std::cout
			<< "knot " << j + 1
			<< ": " << knots[ j ] << std::endl;
		}
        }
	// NOTE: Periodicity is always constant, so periodic and
	// non-periodic curve meshes will have different parts.
	// The curve at i will have numVertices vertices, and may have
	// some knots. The knot count will be numVertices + order for
	// nurbs curves.
	vertexOffset += numVertices * 4;
	knotOffset += numVertices + order;

    }
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

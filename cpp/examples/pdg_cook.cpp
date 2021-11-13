#include <HAPI/HAPI.h>
#include <iostream>
#include <string>
#include <vector>

#include <cassert>

#define ENSURE_SUCCESS(result) \
if ( (result) != HAPI_RESULT_SUCCESS ) \
{ \
    std::cout << "Failure at " << __FILE__ << ": " << __LINE__ << std::endl; \
    std::cout << getLastError() << std::endl; \
    exit( 1 ); \
}

#define ENSURE_COOK_SUCCESS(result) \
if ( (result) != HAPI_RESULT_SUCCESS ) \
{ \
    std::cout << "Failure at " << __FILE__ << ": " << __LINE__ << std::endl; \
    std::cout << getLastCookError() << std::endl; \
    exit( 1 ); \
}

static std::string getLastError();

static std::string getLastCookError();

static std::string getString(HAPI_StringHandle stringHandle);

int
main(int argc, char **argv) {
    const char *hda_file = argc == 2 ? argv[1] : "otls/top_sphere_mountain.hda";

    HAPI_CookOptions cook_options = HAPI_CookOptions_Create();

    HAPI_Session session;

    HAPI_CreateInProcessSession(&session);

    ENSURE_SUCCESS(HAPI_Initialize(&session,
                                   &cook_options,
                                   true,
                                   -1,
                                   nullptr,
                                   nullptr,
                                   nullptr,
                                   nullptr,
                                   nullptr));

    // Load the HDA
    HAPI_AssetLibraryId asset_lib_id;
    ENSURE_SUCCESS(HAPI_LoadAssetLibraryFromFile(&session, hda_file, true, &asset_lib_id));

    int asset_count;
    ENSURE_SUCCESS(HAPI_GetAvailableAssetCount(&session, asset_lib_id, &asset_count));

    HAPI_StringHandle assetsh;
    ENSURE_SUCCESS(HAPI_GetAvailableAssets(&session, asset_lib_id, &assetsh, asset_count));

    std::string asset_name = getString(assetsh);

    HAPI_NodeId asset_node_id;
    ENSURE_SUCCESS(HAPI_CreateNode(&session, -1, asset_name.c_str(), nullptr, true, &asset_node_id));

    // Do a regular node cook
    ENSURE_SUCCESS(HAPI_CookNode(&session, asset_node_id, &cook_options));

    int cook_status;
    HAPI_Result cook_result;
    do {
        cook_result = HAPI_GetStatus(&session, HAPI_STATUS_COOK_STATE, &cook_status);
    } while (cook_status > HAPI_STATE_MAX_READY_STATE && cook_result == HAPI_RESULT_SUCCESS);

    ENSURE_SUCCESS(cook_result);
    ENSURE_COOK_SUCCESS(cook_status);

    // Get the TOP Network node, which is the only child of the asset node
    int network_count = 0;
    ENSURE_SUCCESS(HAPI_ComposeChildNodeList(
            &session, asset_node_id, HAPI_NODETYPE_TOP,
            HAPI_NODEFLAGS_NETWORK, true, &network_count));
    assert(network_count == 1);

    std::vector<HAPI_NodeId> network_ids(network_count);
    ENSURE_SUCCESS(HAPI_GetComposedChildNodeList(
            &session, asset_node_id, network_ids.data(), network_count));

    // Now get the TOP node children of the TOP Network node
    HAPI_NodeId top_network_id = network_ids[0];
    HAPI_NodeInfo node_info;
    ENSURE_SUCCESS(HAPI_GetNodeInfo(&session, top_network_id, &node_info));
    std::string name = getString(node_info.nameSH);
    assert(name == "topnet1");

    // Get all TOP nodes but not schedulers
    int child_count = 0;
    ENSURE_SUCCESS(HAPI_ComposeChildNodeList(
            &session, top_network_id, HAPI_NODETYPE_TOP, HAPI_NODEFLAGS_TOP_NONSCHEDULER,
            true, &child_count));
    assert(child_count == 2);

    std::vector<HAPI_NodeId> child_node_ids(child_count);
    ENSURE_SUCCESS(HAPI_GetComposedChildNodeList(
            &session, top_network_id, child_node_ids.data(), child_count));

    HAPI_NodeId geoimport_id = -1;
    std::string geoimport_name = "geometryimport1";

    // Find ID of the geometry import node. This allows to cook just a particular TOP node, if needed.
    for (HAPI_NodeId child_id : child_node_ids) {
        HAPI_NodeInfo child_node_info;
        ENSURE_SUCCESS(HAPI_GetNodeInfo(&session, child_id, &child_node_info));
        std::string child_name = getString(child_node_info.nameSH);
        std::cout << "TOP node name: " << child_name << std::endl;

        if (child_name.compare(geoimport_name) == 0) {
            geoimport_id = child_id;
        }
    }
    assert(geoimport_id != -1);

    // Do a PDG cook

    // Cook the geometry import TOP node, blocking
    ENSURE_SUCCESS(HAPI_CookPDG(&session, geoimport_id, 0, 1));

    // Query work items after cooking (using new HAPI_GetPDGGraphContextId)

    HAPI_PDG_GraphContextId top_context_id = -1;
    ENSURE_SUCCESS(HAPI_GetPDGGraphContextId(&session, geoimport_id, &top_context_id));

    int num_items = 0;
    ENSURE_SUCCESS(HAPI_GetNumWorkitems(&session, geoimport_id, &num_items));
    assert(num_items == 5);

    HAPI_PDG_WorkitemId *workitem_ids = new HAPI_PDG_WorkitemId[num_items];
    ENSURE_SUCCESS(HAPI_GetWorkitems(&session, geoimport_id, workitem_ids, num_items));

    for (int i = 0; i < num_items; i++) {
        HAPI_PDG_WorkitemInfo workitem_info;
        ENSURE_SUCCESS(HAPI_GetWorkitemInfo(&session, top_context_id, workitem_ids[i], &workitem_info));

        HAPI_PDG_WorkitemResultInfo *result_infos = new HAPI_PDG_WorkitemResultInfo[workitem_info.numResults];
        ENSURE_SUCCESS(HAPI_GetWorkitemResultInfo(&session, geoimport_id, workitem_ids[i], result_infos,
                                                  workitem_info.numResults));

        std::cout << "Result: Tag=" << getString(result_infos[0].resultTagSH) << "; Path="
                  << getString(result_infos[0].resultSH) << std::endl;

        delete[] result_infos;
    }

    delete[] workitem_ids;

    ENSURE_SUCCESS(HAPI_Cleanup(&session));
    ENSURE_SUCCESS(HAPI_CloseSession(&session));

    return 0;
}

static std::string
getLastError() {
    int bufferLength;
    HAPI_GetStatusStringBufLength(nullptr,
                                  HAPI_STATUS_CALL_RESULT,
                                  HAPI_STATUSVERBOSITY_ERRORS,
                                  &bufferLength);

    char *buffer = new char[bufferLength];

    HAPI_GetStatusString(nullptr, HAPI_STATUS_CALL_RESULT, buffer, bufferLength);

    std::string result(buffer);
    delete[] buffer;

    return result;
}

static std::string
getLastCookError() {
    int bufferLength;
    HAPI_GetStatusStringBufLength(nullptr,
                                  HAPI_STATUS_COOK_RESULT,
                                  HAPI_STATUSVERBOSITY_ERRORS,
                                  &bufferLength);

    char *buffer = new char[bufferLength];

    HAPI_GetStatusString(nullptr, HAPI_STATUS_COOK_RESULT, buffer, bufferLength);

    std::string result(buffer);
    delete[] buffer;

    return result;
}

static std::string
getString(HAPI_StringHandle stringHandle) {
    if (stringHandle == 0) {
        return "";
    }

    int bufferLength;
    ENSURE_SUCCESS(HAPI_GetStringBufLength(nullptr,
                                           stringHandle,
                                           &bufferLength));

    if (bufferLength > 0) {
        char *buffer = new char[bufferLength];

        HAPI_GetString(nullptr, stringHandle, buffer, bufferLength);

        std::string result(buffer);
        delete[] buffer;
        return result;
    } else {
        return "";
    }
}
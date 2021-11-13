#include <HAPI/HAPI.h>
#include <iostream>
#include <string>
#include <vector>
#include <thread>

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

    // Cook the geometry import TOP node, in non blocking
    ENSURE_SUCCESS(HAPI_CookPDG(&session, geoimport_id, 0, 0));

    int num_contexts = 0;
    HAPI_StringHandle context_names[20];
    int context_ids[20];

    // While its cooking, check pdg events for each graph context,
    // until cook has finished or errored
    std::vector<HAPI_PDG_EventInfo> pdg_events;
    bool finished = false;
    do {
        std::this_thread::sleep_for(std::chrono::milliseconds(100));

        // Always query the number of graph contexts each time
        ENSURE_SUCCESS(HAPI_GetPDGGraphContexts(&session, &num_contexts,
                                                context_names, context_ids, 20));

        for (int c = 0; c < num_contexts; c++) {
            int cook_context = context_ids[c];

            // Check for new events
            std::vector<HAPI_PDG_EventInfo> event_infos(32);
            int drained = 0, leftOver = 0;
            ENSURE_SUCCESS(HAPI_GetPDGEvents(&session, cook_context,
                                             event_infos.data(), 32, &drained,
                                             &leftOver));

            // Loop over the acquired events
            for (int i = 0; i < drained; i++) {
                switch (event_infos[i].eventType) {
                    case HAPI_PDG_EVENT_WORKITEM_ADD: {
                        break;
                    }
                    case HAPI_PDG_EVENT_WORKITEM_REMOVE: {
                        break;
                    }
                    case HAPI_PDG_EVENT_COOK_WARNING: {
                        break;
                    }
                    case HAPI_PDG_EVENT_COOK_ERROR:
                    case HAPI_PDG_EVENT_COOK_COMPLETE: {
                        finished = true;
                        break;
                    }
                    case HAPI_PDG_EVENT_WORKITEM_STATE_CHANGE: {
                        HAPI_PDG_WorkitemState current_state = (HAPI_PDG_WorkitemState) event_infos[i].currentState;

                        if (current_state == HAPI_PDG_WORKITEM_COOKED_SUCCESS ||
                            current_state == HAPI_PDG_WORKITEM_COOKED_CACHE) {
                            HAPI_PDG_WorkitemInfo workitem_info;
                            ENSURE_SUCCESS(HAPI_GetWorkitemInfo(&session, cook_context, event_infos[i].workitemId,
                                                                &workitem_info));

                            if (workitem_info.numResults > 0) {
                                // Acquire the result tag and path
                                HAPI_PDG_WorkitemResultInfo *result_infos = new HAPI_PDG_WorkitemResultInfo[workitem_info.numResults];
                                ENSURE_SUCCESS(HAPI_GetWorkitemResultInfo(&session, event_infos[i].nodeId,
                                                                          event_infos[i].workitemId, result_infos,
                                                                          workitem_info.numResults));

                                std::cout << "Result: Tag=" << getString(result_infos[0].resultTagSH) << "; Path="
                                          << getString(result_infos[0].resultSH) << std::endl;

                                // Can now load the result, if tagged as file (e.g. Bgeo files can be loaded via HAPI_LoadGeoFromFile)

                                delete[] result_infos;
                            }
                        }
                        break;
                    }
                    default:
                        break;
                }
            }

        }

    } while (!finished);

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
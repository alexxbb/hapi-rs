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

    // Create a TOP network, then a Generic Generator connected to a Text Ouput TOP node.
    // Then create work items, then set and get values.

    HAPI_NodeId topnet_id;
    ENSURE_SUCCESS(HAPI_CreateNode(&session, -1, "Object/topnet", nullptr, true, &topnet_id));

    // Cook it regularly
    ENSURE_SUCCESS(HAPI_CookNode(&session, topnet_id, &cook_options));

    int cook_status;
    HAPI_Result cook_result;

    do {
        cook_result = HAPI_GetStatus(&session, HAPI_STATUS_COOK_STATE, &cook_status);
    } while (cook_status > HAPI_STATE_MAX_READY_STATE && cook_result == HAPI_RESULT_SUCCESS);
    ENSURE_SUCCESS(cook_result);
    ENSURE_COOK_SUCCESS(cook_status);

    HAPI_NodeId generator_id;
    ENSURE_SUCCESS(HAPI_CreateNode(&session, topnet_id, "genericgenerator", nullptr, false, &generator_id));

    HAPI_NodeId textoutput_id;
    ENSURE_SUCCESS(HAPI_CreateNode(&session, topnet_id, "textoutput", nullptr, false, &textoutput_id));

    ENSURE_SUCCESS(HAPI_ConnectNodeInput(&session, textoutput_id, 0, generator_id, 0));

    // Setting the display flag is useful when wanting to cook the TOP network, instead of specific TOP node.
    ENSURE_SUCCESS(HAPI_SetNodeDisplay(&session, textoutput_id, 1));

    ENSURE_SUCCESS(HAPI_CookNode(&session, topnet_id, &cook_options));
    do {
        cook_result = HAPI_GetStatus(&session, HAPI_STATUS_COOK_STATE, &cook_status);
    } while (cook_status > HAPI_STATE_MAX_READY_STATE && cook_result == HAPI_RESULT_SUCCESS);
    ENSURE_SUCCESS(cook_result);
    ENSURE_COOK_SUCCESS(cook_status);

    ENSURE_SUCCESS(HAPI_CookPDG(&session, textoutput_id, 0, 1));

    int num_items = 0;

    // By default, generic generator and text output both have 1 work item each.
    ENSURE_SUCCESS(HAPI_GetNumWorkitems(&session, textoutput_id, &num_items));
    assert(num_items == 1);

    HAPI_ParmId parm_id = -1;

    // Update the text parm on the text output.
    ENSURE_SUCCESS(HAPI_GetParmIdFromName(&session, textoutput_id, "text", &parm_id));
    ENSURE_SUCCESS(HAPI_SetParmStringValue(&session, textoutput_id, "Work item index is `@pdg_index`.", parm_id, 0));

    // Update the item count on the generic generator so that it generates 3 work items.
    ENSURE_SUCCESS(HAPI_SetParmIntValue(&session, generator_id, "itemcount", 0, 3));

    // Don't need to dirty when simply changing the parm value, but dirtying here to remove the cached file
    // results since the text output has been updated.
    ENSURE_SUCCESS(HAPI_DirtyPDGNode(&session, generator_id, true));

    // Cooking will generate files with the above text.
    ENSURE_SUCCESS(HAPI_CookPDG(&session, textoutput_id, 0, 1));

    ENSURE_SUCCESS(HAPI_GetNumWorkitems(&session, textoutput_id, &num_items));
    assert(num_items == 3);

    // Add a work item explicitly to the generic generator
    HAPI_PDG_WorkitemId work_item_id;
    ENSURE_SUCCESS(HAPI_CreateWorkitem(&session, generator_id, &work_item_id, "testwork1", num_items));

    int val = 99;
    float fvals[2] = {2.f, 3.f};
    const char *test_string = "This is a test string!";

    // Just for new work item, set a integer, float array, and string values.
    ENSURE_SUCCESS(HAPI_SetWorkitemIntData(&session, generator_id, work_item_id, "testInt", &val, 1));
    ENSURE_SUCCESS(HAPI_SetWorkitemFloatData(&session, generator_id, work_item_id, "testFloat", fvals, 2));
    ENSURE_SUCCESS(HAPI_SetWorkitemStringData(&session, generator_id, work_item_id, "testString", 0, test_string));

    ENSURE_SUCCESS(HAPI_CommitWorkitems(&session, generator_id));

    ENSURE_SUCCESS(HAPI_GetNumWorkitems(&session, generator_id, &num_items));
    assert(num_items == 4);

    int datalen = 0;
    val = 0;
    fvals[0] = fvals[1] = 0;

    // Get work item integer value
    ENSURE_SUCCESS(HAPI_GetWorkitemDataLength(&session, generator_id, work_item_id, "testInt", &datalen));
    assert(datalen == 1);
    ENSURE_SUCCESS(HAPI_GetWorkitemIntData(&session, generator_id, work_item_id, "testInt", &val, 1));
    assert(val == 99);

    // Get work item float value
    ENSURE_SUCCESS(HAPI_GetWorkitemFloatData(&session, generator_id, work_item_id, "testFloat", fvals, 2));
    assert(fvals[0] == 2.f && fvals[1] == 3.f);

    // Get work item string value
    ENSURE_SUCCESS(HAPI_GetWorkitemDataLength(&session, generator_id, work_item_id, "testString", &datalen));
    assert(datalen == 1);

    HAPI_StringHandle str_handle;
    ENSURE_SUCCESS(
            HAPI_GetWorkitemStringData(&session, generator_id, work_item_id, "testString", &str_handle, datalen));
    ENSURE_SUCCESS(HAPI_GetStringBufLength(&session, str_handle, &datalen));
    assert(datalen == strlen(test_string) + 1);

    std::vector<char> stringData(datalen + 1);
    ENSURE_SUCCESS(HAPI_GetString(&session, str_handle, stringData.data(), datalen));
    assert(strcmp(stringData.data(), test_string) == 0);

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
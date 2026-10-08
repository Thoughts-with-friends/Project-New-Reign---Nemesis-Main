#include "EngineMain.h"

#if _WIN32
int wmain(int argc, wchar_t* argv[])
#else
int main(int argc, char* argv[])
#endif
{
    return nemesis_engine_main(argc, argv);
}

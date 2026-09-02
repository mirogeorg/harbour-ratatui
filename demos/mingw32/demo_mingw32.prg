PROCEDURE Main()
   LOCAL cScreen
   LOCAL aFrame

   IF ! RTUI_AVAILABLE()
      ? "Ratatui is unavailable:", RTUI_LAST_ERROR()
      ErrorLevel( 1 )
      RETURN
   ENDIF

   aFrame := RTUI_FRAME_NEW( 72, 15 )
   RTUI_FRAME_PARAGRAPH( aFrame, RTUI_RECT( 0, 0, 72, 11 ), "MinGW 32 demo", ;
      "Compiler: Harbour + MinGW i686" + Chr( 10 ) + ;
      "This 32-bit process uses the same generic HRC1 interface.", ;
      RTUI_RGB( 240, 244, 255 ), RTUI_RGB( 18, 24, 38 ), ;
      RTUI_RGB( 190, 120, 255 ), 0, .T., .F., .T. )
   RTUI_FRAME_GAUGE( aFrame, RTUI_RECT( 0, 11, 72, 4 ), "Binding status", ;
      "Harbour command buffer -> Ratatui: OK", 1.0, ;
      RTUI_RGB( 190, 120, 255 ), RTUI_RGB( 38, 32, 55 ), ;
      RTUI_RGB( 190, 120, 255 ) )
   cScreen := RTUI_FRAME_RENDER( aFrame, .T. )

   IF cScreen == NIL
      ? "Render failed:", RTUI_LAST_ERROR()
      ErrorLevel( 2 )
      RETURN
   ENDIF

   IF ! RTUI_PRESENT( cScreen, .T. )
      ? "Native console presentation failed:", RTUI_LAST_ERROR()
      ErrorLevel( 3 )
      RETURN
   ENDIF
   ? "Loaded ABI version:", RTUI_ABI_VERSION()
RETURN

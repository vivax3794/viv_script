; ModuleID = 'TMP'
source_filename = "TMP"

@I32_Print_Format_String = private unnamed_addr constant [4 x i8] c"%d\0A\00", align 1
@I32_Print_Format_String.1 = private unnamed_addr constant [4 x i8] c"%d\0A\00", align 1

declare i32 @printf(i8*, ...)

define i32 @main() {
entry:
  %Print_Statement = call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @I32_Print_Format_String, i32 0, i32 0), i32 6)
  %Print_Statement1 = call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @I32_Print_Format_String.1, i32 0, i32 0), i32 489)
  ret i32 0
}

; ModuleID = 'TMP'
source_filename = "TMP"

@Literal_String = private unnamed_addr constant [15 x i8] c"Hello Matisse!\00", align 1
@Print_Format_String = private unnamed_addr constant [4 x i8] c"%s\0A\00", align 1

declare i32 @printf(i8*, ...)

define i32 @main() {
entry:
  %Print_Statement = call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([4 x i8], [4 x i8]* @Print_Format_String, i32 0, i32 0), [15 x i8]* @Literal_String)
  ret i32 0
}

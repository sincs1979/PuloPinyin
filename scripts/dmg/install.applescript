on run
	set appPath to POSIX path of (path to me)
	set here to do shell script "dirname " & quoted form of appPath
	try
		do shell script "xattr -cr " & quoted form of here
	end try
	set cmd to here & "/安装.command"
	do shell script "bash " & quoted form of cmd
end run

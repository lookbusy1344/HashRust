# Powershell wrapper around the rust_hash.exe binary
# This will parse the output into useful objects

# call like this:
# $result = .\hashfile.ps1 "C:\Users\user\Downloads\*.exe" -algorithm sha3



[CmdletBinding(DefaultParameterSetName = 'Parameter Set 1',
    PositionalBinding = $false,
    ConfirmImpact = 'Medium')]

param (
    # a search glob passed direct to the exe, eg '*.txt'
    [Parameter(Mandatory = $false, ParameterSetName = 'Parameter Set 1', Position = 0)][string]$path,

    # files passed as pipeline objects, eg dir | sha3.ps1
    [parameter(Mandatory = $false, ParameterSetName = 'Parameter Set 1', 
        ValueFromPipeline = $true, ValueFromPipelineByPropertyName = $true)]
    [string[]]$inputfile,

    # algorithm, eg MD5, SHA1, SHA256, SHA384, SHA512, SHA3-256, SHA3-384, SHA3-512
    [string]$algorithm = "SHA3",

    # limit the number of files to hash
    [int]$limit = 0
)

begin {
    # runs once at start of script

    class FileHashJS {
        [string]$Hash
        [System.IO.FileInfo]$File

        FileHashJS(
            [string]$line
        ) {
            $split = $line.Split(' ', 2)
            $this.Hash = $split[0]
            $this.File = [System.IO.FileInfo] $split[1]
        }
    }

    $pipelinelist = New-Object Collections.Generic.List[string]
}

process {
    # run this for every pipeline object
    if ($inputfile) {
        $pipelinelist.Add($inputfile)
    }
}

end {
    # runs once at end of script. This does all the work

    $piped = $pipelinelist.Count -gt 0

    if (-not $piped -and $path.Length -eq 0) {
        throw "No path or piped input"
    }

    if ($limit -gt 0) {
        # a limit is set
        if ($piped) {
            $raw = $pipelinelist | hash_rust.exe --limit $limit --algorithm $algorithm
        }
        else {
            $raw = hash_rust.exe --limit $limit --algorithm $algorithm $path
        }
    }
    else {
        if ($piped) {
            $raw = $pipelinelist | hash_rust.exe --algorithm $algorithm
        }
        else {
            $raw = hash_rust.exe --algorithm $algorithm $path
        }
    }

    # turn the raw output into powershell objects
    $results = $raw | ForEach-Object { [FileHashJS]::new($_) }

    $results
}
